//! A World's own window: its view and status, finding and focusing the
//! windows already open onto a World, asking before closing or quitting
//! with unsaved Worlds, and what the window does (save, reload, save as,
//! a new day at midnight, sharing, the demo's farewell). Moved out of
//! `main.rs` as it stood.

#[allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DocumentStatusTone {
    Success,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DocumentStatus {
    pub(crate) message: String,
    pub(crate) tone: DocumentStatusTone,
}

impl DocumentStatus {
    pub(crate) fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: DocumentStatusTone::Success,
        }
    }

    pub(crate) fn error(message: impl Into<String>) -> Self {
        let message = message.into();
        diagnostics::error(format!("document: {message}"));
        Self {
            message,
            tone: DocumentStatusTone::Error,
        }
    }
}

pub(crate) struct WorldDocumentView {
    /// The durable identity of the World's file. Stays visible so a World can
    /// always be matched to the file it lives in.
    pub(crate) document_label: String,
    /// What this World is called: the name its owner gave it on Home, or the
    /// durable identity when it has none.
    pub(crate) document_name: String,
    pub(crate) document: SharedDocument,
    pub(crate) projection: Entity<world_gpui::ProjectionView>,
    pub(crate) status: Option<DocumentStatus>,
    /// When, in Unix seconds, this World next moves on its own; read once
    /// when it opens, since it only moves between visits.
    pub(crate) next_move_at: Option<u64>,
    /// Whether the Share list under the title bar is open.
    pub(crate) share_open: bool,
    /// Whether the app menu under the title bar is open, the item the
    /// keyboard is on, and what holds the keyboard while it is open.
    pub(crate) menu_open: bool,
    pub(crate) menu_focus: usize,
    pub(crate) menu_keys: gpui::FocusHandle,
    /// What the demo asks this window to show next (set by the controller
    /// when it turned a day's passing; always empty in the full app).
    pub(crate) demo_cue: DemoCue,
    /// Set when the player chose to close this window although its World's
    /// latest turns could not be written (see [`Self::ready_to_close`]).
    pub(crate) closing_anyway: bool,
    /// Wakes the window at the next local midnight, if it is left open and
    /// active across it (v0.30 S4): dropped with the window.
    pub(crate) midnight: Option<gpui::Task<()>>,
}

/// A World's own view in its window: the scene with no title bar of its
/// own (the window has one), and a handle that shows it as a strip.
/// "Sleep now", playing freely, asks `host` to begin the next sitting.
pub(crate) fn world_view(
    controller: HostProjectionController,
    host: gpui::WeakEntity<WorldDocumentView>,
) -> world_gpui::ProjectionView {
    // The sitting's days, as its ledger has them now (v0.30 S4): every
    // window onto the World, its strip's too, shares them. The ledger is
    // the World file's, written through the World's own session.
    let document = Rc::clone(&controller.document);
    let sitting = document.borrow().sitting.clone();
    let view = world_gpui::ProjectionView::controlled(controller)
        .without_header()
        .with_strip(|window, cx| window.dispatch_action(Box::new(about::ShowAsStrip), cx));
    // A ledger that cannot be read gives one day, never days without end.
    let one_day = world_gpui::Allowance { left: 1, of: 1 };
    match sitting {
        None => view,
        Some(SittingKept::Unreadable) => view.with_allowance(one_day, || {}),
        Some(SittingKept::Ledger(ledger)) => {
            let (allowance, read) = {
                let state = document.borrow();
                let now = ledger.sitting(&state.session);
                (
                    world_gpui::Allowance {
                        left: now.left,
                        of: now.allowance,
                    },
                    ledger.morning_read(&state.session),
                )
            };
            let reads = ledger.clone();
            let reading = Rc::clone(&document);
            view.with_morning_read(read.as_deref(), move |moment| {
                match reading.try_borrow_mut() {
                    Ok(mut state) => {
                        let state = &mut *state;
                        reads.read_morning(&mut state.session, &state.library, moment);
                    }
                    Err(_) => diagnostics::error("could not write the morning down: busy"),
                }
            })
            .with_allowance(allowance, move || match document.try_borrow_mut() {
                Ok(mut state) => {
                    let state = &mut *state;
                    ledger.spend_day(&mut state.session, &state.library);
                }
                Err(_) => diagnostics::error("could not write the sitting down: busy"),
            })
            .with_sleep_now(move |cx| {
                let _ = host.update(cx, |host, cx| host.sleep_now(cx));
            })
        }
    }
}

/// `view`'s window asks it before closing (see
/// [`WorldDocumentView::ready_to_close`]).
pub(crate) fn ask_before_closing(
    view: Entity<WorldDocumentView>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<WorldDocumentView> {
    let asked = view.downgrade();
    window.on_window_should_close(cx, move |window, cx| {
        asked
            .update(cx, |view, cx| view.ready_to_close(window, cx))
            .unwrap_or(true)
    });
    // A World asleep in a window left open overnight wakes when it is next
    // brought forward (by Home, its strip, or a click), on a new sitting
    // (v0.30 S4); and one left active across midnight, at midnight. A
    // World still awake is never woken (1.0 P2): its day and evening go
    // on past midnight, and the new day comes once Goodnight is said.
    view.update(cx, |view, cx| {
        view.arm_midnight(cx);
        cx.observe_window_activation(window, |this, window, cx| {
            if window.is_window_active() {
                this.wake_if_asleep_on_a_new_day(cx);
            }
        })
        .detach();
    });
    view
}

/// The World `id` as it is open in this app (in a window or a strip), if it
/// is: Home renames, exports and removes an open World through its session,
/// never behind its back.
pub(crate) fn open_world_document(cx: &App, id: &WorldDocumentId) -> Option<SharedDocument> {
    open_world_where(cx, |session| session.document_id() == Some(id))
}

/// The open World whose session `is` says it is the one.
pub(crate) fn open_world_where(
    cx: &App,
    is: impl Fn(&DurableWorldSession) -> bool,
) -> Option<SharedDocument> {
    let in_windows = cx.windows().into_iter().filter_map(|window| {
        let view = window.downcast::<WorldDocumentView>()?;
        view.read(cx).ok().map(|view| Rc::clone(&view.document))
    });
    in_windows
        .chain(strip_window::documents())
        .find(|document| {
            document
                .try_borrow()
                .is_ok_and(|document| is(&document.session))
        })
}

/// Redraws every window onto `document` with its name as it is now.
pub(crate) fn refresh_world_windows(document: &SharedDocument, cx: &mut App) {
    for window in cx.windows() {
        let Some(view) = window.downcast::<WorldDocumentView>() else {
            continue;
        };
        let _ = view.update(cx, |view, _, cx| {
            if Rc::ptr_eq(&view.document, document) {
                view.refresh_document_identity();
                cx.notify();
            }
        });
    }
}

/// Brings the window of the World `id` to the front, if it is open in one.
pub(crate) fn focus_open_world(cx: &mut App, id: &WorldDocumentId) -> bool {
    let document = open_world_document(cx, id);
    focus_world(cx, document)
}

/// Brings the window of `document` to the front; `false` if there is none.
pub(crate) fn focus_world(cx: &mut App, document: Option<SharedDocument>) -> bool {
    let Some(document) = document else {
        return false;
    };
    let window = cx.windows().into_iter().find_map(|window| {
        let view = window.downcast::<WorldDocumentView>()?;
        view.read(cx)
            .is_ok_and(|view| Rc::ptr_eq(&view.document, &document))
            .then_some(view)
    });
    match window {
        Some(window) => {
            let _ = window.update(cx, |_, window, _| window.activate_window());
        }
        // Shown only as a strip: its window opens again from the strip.
        None => strip_window::show_world(&document, cx),
    }
    true
}

/// Every World open in this app, each once: its name, and why its latest
/// turns could not be written, for those that could not.
pub(crate) fn unsaved_worlds(cx: &App) -> Vec<(String, String)> {
    let mut seen: Vec<SharedDocument> = Vec::new();
    let in_windows = cx.windows().into_iter().filter_map(|window| {
        let view = window.downcast::<WorldDocumentView>()?;
        view.read(cx).ok().map(|view| Rc::clone(&view.document))
    });
    for document in in_windows.chain(strip_window::documents()) {
        if !seen.iter().any(|known| Rc::ptr_eq(known, &document)) {
            seen.push(document);
        }
    }
    let mut failures = seen
        .iter()
        .filter_map(|document| {
            let document = document.try_borrow().ok()?;
            let error = document.session.flush().err()?;
            Some((session_display_name(&document.session), error.to_string()))
        })
        .collect::<Vec<_>>();
    // Any World still being let go of, with no window left to name it.
    if failures.is_empty() {
        failures.extend(
            world_library::flush_all_writes()
                .into_iter()
                .map(|error| ("A World".to_string(), error.to_string())),
        );
    }
    failures
}

/// Quits once every open World's latest turns are written; if some cannot
/// be, asks first, and keeps the app running unless told to quit anyway
/// (v0.26 only logged them, and quit).
pub(crate) fn quit_after_saving(cx: &mut App) {
    let failures = unsaved_worlds(cx);
    if failures.is_empty() {
        cx.quit();
        return;
    }
    for (name, why) in &failures {
        diagnostics::error(format!("saving {name} as the app quits: {why}"));
    }
    let (message, detail) = saving::quit_failure(&failures);
    let Some(window) = cx.active_window().or_else(|| cx.windows().first().copied()) else {
        cx.quit();
        return;
    };
    let asked = window.update(cx, |_, window, cx| {
        let answer = window.prompt(
            gpui::PromptLevel::Critical,
            &message,
            Some(&detail),
            &saving::QUIT_CHOICES,
            cx,
        );
        cx.spawn(async move |cx| {
            if answer.await == Ok(saving::QUIT_ANYWAY) {
                cx.update(|cx| cx.quit());
            }
        })
        .detach();
    });
    if asked.is_err() {
        cx.quit();
    }
}

impl WorldDocumentView {
    pub(crate) fn new(
        session: DurableWorldSession,
        registry: Arc<world_host::WorldRegistry>,
        library: Arc<WorldLibrary>,
        sitting: Option<SittingKept>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::with_document(
            Rc::new(RefCell::new(SharedDocumentState {
                session,
                registry,
                library,
                sitting,
            })),
            cx,
        )
    }

    /// A window onto a World that is already open elsewhere, such as in a
    /// strip along the edge of the screen.
    pub(crate) fn with_document(document: SharedDocument, cx: &mut Context<Self>) -> Self {
        let (document_label, document_name, next_move_at) = {
            let state = document.borrow();
            (
                state.session.display_name(),
                session_display_name(&state.session),
                // Its next day comes with the next sitting: tomorrow.
                state
                    .sitting
                    .as_ref()
                    .and_then(|_| observer::next_sitting_in())
                    .zip(unix_now())
                    .map(|(remaining, now)| now + remaining),
            )
        };
        let demo_cue: DemoCue = Rc::new(std::cell::Cell::new(None));
        let controller = HostProjectionController::new(Rc::clone(&document), Rc::clone(&demo_cue));
        let host = cx.weak_entity();
        let projection = cx.new(|_| world_view(controller, host));
        // The title bar reads the World's name and what it can do from the
        // page, so it redraws whenever the page does.
        Self::follow_projection(&projection, cx);
        // Closing the last World brings Home back, even with Settings or
        // another small window still open, and silences its sound.
        let sound_owner = cx.entity_id().as_u64();
        cx.on_release(move |_, cx| {
            ambience::player::release(sound_owner);
            cx.defer(restore_home_if_nothing_open);
        })
        .detach();
        Self {
            document_label,
            document_name,
            document,
            projection,
            status: None,
            next_move_at,
            share_open: false,
            menu_open: false,
            menu_focus: 0,
            menu_keys: cx.focus_handle(),
            demo_cue,
            closing_anyway: false,
            midnight: None,
        }
    }

    /// Asked before this window closes: the World's changes are written
    /// first, and if they cannot be, the window stays open and asks what to
    /// do (v0.26 let a failed last write go with the window, in silence).
    pub(crate) fn ready_to_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.closing_anyway {
            return true;
        }
        let flushed = match self.document.try_borrow() {
            Ok(document) => document.session.flush(),
            // Mid-turn: the turn's own error, if any, is shown by the turn.
            Err(_) => Ok(()),
        };
        let Err(error) = flushed else {
            return true;
        };
        let (message, detail) = saving::close_failure(&self.document_name, &error);
        self.status = Some(DocumentStatus::error(format!("{message}. {detail}")));
        let answer = window.prompt(
            gpui::PromptLevel::Critical,
            &message,
            Some(&detail),
            &saving::CLOSE_CHOICES,
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let Ok(choice) = answer.await else {
                return;
            };
            let _ = this.update_in(cx, |this, window, cx| match choice {
                saving::SAVE_AS => this.save_as(cx),
                saving::CLOSE_ANYWAY => {
                    diagnostics::error(format!(
                        "closed {} without saving its latest turns",
                        this.document_name
                    ));
                    this.closing_anyway = true;
                    window.remove_window();
                }
                _ => {}
            });
        })
        .detach();
        cx.notify();
        false
    }

    /// File → Close on a World's window: closes it once its World is
    /// written, as the window's own close button does.
    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.ready_to_close(window, cx) {
            window.remove_window();
        }
    }

    /// Re-read what this World is called from the session, after anything
    /// that can change its file or its target.
    pub(crate) fn refresh_document_identity(&mut self) {
        let (label, name) = {
            let document = self.document.borrow();
            (
                document.session.display_name(),
                session_display_name(&document.session),
            )
        };
        self.document_label = label;
        self.document_name = name;
    }

    pub(crate) fn reload(&mut self, cx: &mut Context<Self>) {
        let result = {
            let mut document = self.document.borrow_mut();
            let registry = Arc::clone(&document.registry);
            let library = Arc::clone(&document.library);
            document.session.reload(&registry, &library)
        };

        match result {
            Ok(snapshot) => {
                if self.document.borrow().session.document_id().is_some() {
                    mark_library_changed();
                }
                self.refresh_document_identity();
                self.rebuild_projection(cx);
                self.status = Some(DocumentStatus::success(format!(
                    "Reloaded {} · {}",
                    self.document_name,
                    world_gpui::i18n::moment_label(&snapshot, snapshot.world_time)
                )));
            }
            Err(error) => {
                self.status = Some(DocumentStatus::error(format!("Reload failed: {error}")));
            }
        }
        cx.notify();
    }

    pub(crate) fn save_as(&mut self, cx: &mut Context<Self>) {
        let semantic_title = self.document.borrow().session.snapshot().title;
        let suggested_name = suggested_world_file_name(&semantic_title, &self.document_name);
        let save_dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested_name));
        cx.spawn(async move |this, cx| {
            let mut without_dialog = false;
            let destination = match save_dialog.await {
                Ok(Ok(Some(path))) => canonical_world_path(path),
                Ok(Ok(None)) => return,
                // No Save window on this computer: the World is saved in
                // Documents anyway, and the title bar says where.
                Ok(Err(error)) => {
                    diagnostics::error(format!("save as dialog: {error}"));
                    without_dialog = true;
                    let folder = world_machine_desktop::plain_errors::fallback_folder();
                    match folder {
                        Ok(folder) => {
                            canonical_world_path(world_machine_desktop::plain_errors::free_path(
                                &folder,
                                &suggested_name,
                            ))
                        }
                        Err(error) => {
                            diagnostics::error(format!("save as: {error}"));
                            let _ = this.update(cx, |this, cx| {
                                this.status = Some(DocumentStatus::error(
                                    world_machine_desktop::plain_errors::not_saved_without_dialog(),
                                ));
                                cx.notify();
                            });
                            return;
                        }
                    }
                }
                Err(error) => {
                    diagnostics::error(format!("save as dialog: {error}"));
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(DocumentStatus::error(
                            world_machine_desktop::plain_errors::file_window_closed(),
                        ));
                        cx.notify();
                    });
                    return;
                }
            };

            let _ = this.update(cx, |this, cx| {
                let result = {
                    let mut document = this.document.borrow_mut();
                    document.session.save_as_file(destination.clone())
                };
                match result {
                    Ok(snapshot) => {
                        this.refresh_document_identity();
                        // The sitting went with the World to its new file.
                        this.rebuild_projection(cx);
                        this.status = Some(DocumentStatus::success(if without_dialog {
                            world_machine_desktop::plain_errors::saved_without_dialog(&destination)
                        } else {
                            format!(
                                "Saved As {} · {}",
                                this.document_name,
                                world_gpui::i18n::moment_label(&snapshot, snapshot.world_time)
                            )
                        }));
                    }
                    Err(error) => {
                        diagnostics::error(format!("save as: {error}"));
                        this.status = Some(DocumentStatus::error(if without_dialog {
                            world_machine_desktop::plain_errors::not_saved_without_dialog()
                        } else {
                            world_machine_desktop::plain_errors::could_not(
                                "Couldn't save this World.",
                                &error.to_string(),
                            )
                        }));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A new sitting, if one has begun since this window's (a window left
    /// open overnight, or a clock put right): it is written down, the World
    /// told, and the page asked anew, so the window wakes with the new
    /// day's allowance and its morning.
    /// Wakes the window at the next local midnight, and again at every one
    /// after it, for as long as the window stays open: a window active
    /// across midnight has no activation to wake it. The task goes with the
    /// window.
    pub(crate) fn arm_midnight(&mut self, cx: &mut Context<Self>) {
        let has_sitting = matches!(
            &self.document.borrow().sitting,
            Some(SittingKept::Ledger(_))
        );
        let Some(wait) = observer::wake_wait().filter(|_| has_sitting) else {
            self.midnight = None;
            return;
        };
        self.midnight = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            let _ = this.update(cx, |this, cx| {
                this.wake_if_asleep_on_a_new_day(cx);
                this.arm_midnight(cx);
            });
        }));
    }

    /// The title bar follows the page; and a World that falls asleep once
    /// a new day of the player's has already begun (Goodnight said past
    /// midnight) wakes to it.
    pub(crate) fn follow_projection(
        projection: &Entity<world_gpui::ProjectionView>,
        cx: &mut Context<Self>,
    ) {
        cx.observe(projection, |this, projection, cx| {
            if projection.read(cx).asleep() {
                this.wake_if_asleep_on_a_new_day(cx);
            }
            cx.notify();
        })
        .detach();
    }

    /// Wakes the window on a new sitting if its World is asleep and a new
    /// day of the player's has begun. A World still awake (its day being
    /// played, or its evening) is let be: nothing ends a day on a clock.
    pub(crate) fn wake_if_asleep_on_a_new_day(&mut self, cx: &mut Context<Self>) {
        if !self.projection.read(cx).asleep() {
            return;
        }
        let due = matches!(
            &self.document.borrow().sitting,
            Some(SittingKept::Ledger(ledger)) if ledger.new_day_due()
        );
        if due {
            self.begin_sitting(observer::sit, cx);
        }
    }

    /// "Sleep now" (1.0 P2: playing freely): the next sitting begins at
    /// once, and the window opens on its morning.
    pub(crate) fn sleep_now(&mut self, cx: &mut Context<Self>) {
        if !matches!(
            &self.document.borrow().sitting,
            Some(SittingKept::Ledger(_))
        ) {
            return;
        }
        self.begin_sitting(observer::sleep_now, cx);
    }

    /// Begins a sitting with `sit` (opening's, or Sleep now's): written
    /// down, the World told, and the page asked anew, so the window wakes
    /// with the new day's allowance and its morning.
    fn begin_sitting(
        &mut self,
        sit: fn(
            &mut DurableWorldSession,
            &world_host::WorldRegistry,
            &WorldLibrary,
        ) -> Result<observer::Sat, String>,
        cx: &mut Context<Self>,
    ) {
        let sat = {
            let Ok(mut state) = self.document.try_borrow_mut() else {
                return;
            };
            let SharedDocumentState {
                session,
                registry,
                library,
                sitting,
            } = &mut *state;
            match sit(session, registry, library) {
                Ok(sat) => {
                    *sitting = Some(SittingKept::Ledger(sat.ledger()));
                    true
                }
                Err(error) => {
                    diagnostics::error(format!("could not begin a new sitting: {error}"));
                    false
                }
            }
        };
        if sat {
            self.next_move_at = observer::next_sitting_in()
                .zip(unix_now())
                .map(|(remaining, now)| now + remaining);
            self.rebuild_projection(cx);
            cx.notify();
        }
    }

    pub(crate) fn rebuild_projection(&mut self, cx: &mut Context<Self>) {
        let controller =
            HostProjectionController::new(Rc::clone(&self.document), Rc::clone(&self.demo_cue));
        let host = cx.weak_entity();
        self.projection = cx.new(|_| world_view(controller, host));
        // The title bar, and the demo's turns, follow the new page too.
        Self::follow_projection(&self.projection, cx);
    }

    /// Share, under the title bar: the World's code, copied or saved, and
    /// a guest from another World; the same as the World menu's items.
    pub(crate) fn share_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let item = |id: &'static str, label: &'static str| {
            let label = ui::t(label);
            div()
                .id(id)
                .role(gpui::Role::MenuItem)
                .aria_label(label.clone())
                .px_3()
                .py(px(6.0))
                .rounded_md()
                .text_sm()
                .text_color(ui::color(tokens::TEXT))
                .cursor_pointer()
                .hover(|style| style.bg(ui::color(tokens::ROW_HOVER)))
                .child(label)
        };
        div()
            .id("share-list")
            .role(gpui::Role::Menu)
            .aria_label(ui::t("Share"))
            .absolute()
            .top(px(48.0))
            .right(px(16.0))
            .w(px(240.0))
            .p_1()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .shadow_lg()
            .child(
                item("share-copy-code", "Copy World Code").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.share_open = false;
                        this.copy_world_code(cx);
                    },
                )),
            )
            .child(
                item("share-save-code", "Save World Code…").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.share_open = false;
                        this.save_world_code(cx);
                    },
                )),
            )
            .child(
                item("share-invite-guest", "Invite a Guest").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.share_open = false;
                        this.invite_guest(cx);
                    },
                )),
            )
            .child(
                item("share-invite-friend", "Invite a Friend's Resident…").on_click(
                    cx.listener(|this, _, _, cx| {
                        this.share_open = false;
                        this.invite_friend(cx);
                    }),
                ),
            )
            .child(div().px_3().pt_1().pb(px(6.0)).child(ui::caption(
                "Anyone can open a World code as a visit. A guest comes from another of your Worlds, or from a friend's World code.",
            )))
    }

    /// Opens or closes the app menu. Open, it holds the keyboard (the
    /// arrows, Enter and Esc); closed, the scene takes the keyboard back.
    pub(crate) fn toggle_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu_open = !self.menu_open;
        self.menu_focus = 0;
        self.share_open = false;
        if self.menu_open {
            window.focus(&self.menu_keys, cx);
        } else {
            window.blur(cx);
        }
        cx.notify();
    }

    /// An item of the app menu: the World's own (Save As, its code) here,
    /// the app's through the menu bar's action.
    pub(crate) fn pick(
        &mut self,
        item: shell::MenuItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.menu_open = false;
        window.blur(cx);
        if !app_menu::run_app_item(item, window, cx) {
            match item {
                shell::MenuItem::SaveAs => self.save_as(cx),
                shell::MenuItem::CopyWorldCode => self.copy_world_code(cx),
                shell::MenuItem::SaveWorldCode => self.save_world_code(cx),
                _ => {}
            }
        }
        cx.notify();
    }

    fn menu_key(
        &mut self,
        event: &gpui::KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.menu_open {
            return;
        }
        let keystroke = &event.keystroke;
        let key = keystroke.key.as_str();
        let items = shell::app_menu(shell::MenuPlace::World);
        if key == "escape" {
            self.toggle_menu(window, cx);
        } else if let Some(step) = shell::step_for_key(key, keystroke.modifiers.shift) {
            self.menu_focus = shell::step(items.len(), self.menu_focus, step);
            cx.notify();
        } else if shell::chooses(key) {
            if let Some(item) = items.get(self.menu_focus).copied() {
                self.pick(item, window, cx);
            }
        } else {
            return;
        }
        cx.stop_propagation();
    }

    /// The app menu, under its button at the title bar's right.
    fn app_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pick: app_menu::Pick<Self> =
            Rc::new(|this: &mut Self, item, window, cx| this.pick(item, window, cx));
        div()
            .id("world-app-menu")
            .track_focus(&self.menu_keys)
            .on_key_down(cx.listener(Self::menu_key))
            .absolute()
            .top(px(48.0))
            .right(px(16.0))
            .child(app_menu::menu_list(
                &shell::app_menu(shell::MenuPlace::World),
                Some(self.menu_focus),
                pick,
                cx,
            ))
    }

    /// The demo's ending: the wishlist card over the World, at dusk,
    /// recapping the player's own moments from what the World recorded,
    /// with a resident's goodbye, a postcard to keep and "Wishlist ... on
    /// Steam" (the game named by its Pack). "Stay a while" puts it away,
    /// and it comes back only when the next day is asked for again.
    pub(crate) fn show_demo_farewell(&mut self, cx: &mut Context<Self>) {
        let game = {
            let document = self.document.borrow();
            let pack = document.session.pack();
            document
                .registry
                .descriptor_for(&pack)
                .or_else(|| document.registry.descriptor(&pack.id))
                .map(|descriptor| descriptor.title.clone())
                .unwrap_or_else(|| document.session.snapshot().title)
        };
        self.projection.update(cx, |view, cx| {
            let snapshot = view.snapshot().clone();
            let goodbye = demo::goodbye_from(&snapshot).map(|who| (who, demo::GOODBYE.to_string()));
            // The recap in the World's own words, which the farewell shows
            // in the player's language.
            let recorded = self.document.borrow().session.snapshot();
            view.show_farewell(world_gpui::Farewell {
                title: demo::ENDING_TITLE.into(),
                recap_title: demo::RECAP_TITLE.into(),
                recap: demo::recap(&recorded),
                goodbye,
                body: demo::ENDING_BODY.into(),
                kept: demo::ENDING_KEPT.into(),
                postcard: demo::KEEP_POSTCARD.into(),
                more: Some((demo::wishlist_label(&game), wishlist())),
                stay: demo::ENDING_STAY.into(),
                hour: demo::FAREWELL_HOUR,
            });
            cx.notify();
        });
    }
}

/// What "Wishlist ... on Steam" does from the demo's closing card: opens
/// the store page in the player's browser (`demo::WISHLIST_URL`, a
/// placeholder until the owner gives the game's page). Nothing is sent.
pub(crate) fn wishlist() -> world_gpui::FarewellAction {
    std::rc::Rc::new(|_: &mut Window, cx: &mut gpui::App| cx.open_url(demo::WISHLIST_URL))
}

impl Render for WorldDocumentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        remember_window_geometry(window, RememberedWindow::World);
        // A World's name follows it as it changes ("A new World" becomes
        // the name its Pack gives it when it is seeded), so read it every time.
        self.document_name = session_display_name(&self.document.borrow().session);
        window.set_window_title(&document_window_title(&self.document_name));
        // The World in front plays its landscape and its music, if the
        // player wants sound; one behind fades out.
        let sound_owner = cx.entity_id().as_u64();
        let scene = {
            let snapshot = self.projection.read(cx).snapshot();
            snapshot.scenery.map(|scenery| {
                ambience::scene(
                    &self.document.borrow().session.pack().id,
                    &snapshot.title,
                    [
                        scenery.sky_top,
                        scenery.sky_bottom,
                        scenery.far,
                        scenery.near,
                        scenery.sun,
                    ],
                    world_gpui::scene::hour_now(),
                    ambience::sky(snapshot.weather),
                    snapshot
                        .calendar
                        .as_ref()
                        .is_some_and(|calendar| calendar.festival_today),
                )
            })
        };
        // A strip of this World shows it as it is now.
        strip_window::follow(&self.document, &self.projection, cx);
        match (window.is_window_active(), scene) {
            (true, Some(scene)) => ambience::player::claim(sound_owner, scene),
            _ => ambience::player::release(sound_owner),
        }
        if let Some(line) = ambience::player::take_report() {
            diagnostics::info(line);
        }
        // Beside its name, the one thing the app is about: this World goes on
        // without you, and when it next will.
        // Only a World that moves on its own may promise to keep going.
        let (moves_alone, unit) = {
            let snapshot = self.projection.read(cx).snapshot();
            (
                snapshot.capabilities.background,
                snapshot
                    .calendar
                    .as_ref()
                    .map(|calendar| calendar.unit.to_lowercase())
                    .unwrap_or_else(|| "day".into()),
            )
        };
        // Nor while the demo's farewell shows: there is no next day there.
        let farewell = self.projection.read(cx).farewell_shown();
        // Heading home under the Goodnight card, asleep once it is said
        // (v0.30 S4).
        let rest = self.projection.read(cx).rest();
        let freely = world_gpui::ui::play_freely();
        let keeps_going = self
            .next_move_at
            .filter(|_| moves_alone && !farewell)
            .zip(unix_now())
            .map(|(at, now)| keeps_going_line(at.saturating_sub(now), &unit, rest, freely));
        // The World is called by its name, and only by its name; which file
        // it lives in is for Export and Show in Finder to say.
        let identity = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .gap_2()
            .items_center()
            .overflow_hidden()
            .child(
                div()
                    .flex_shrink_0()
                    .text_base()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .truncate()
                    .child(ui::t(self.document_name.clone())),
            )
            .when_some(keeps_going, |identity, line| {
                identity.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .min_w(px(0.0))
                        .child(
                            div()
                                .flex_shrink_0()
                                .size(px(6.0))
                                .rounded_full()
                                .bg(ui::color(tokens::SUCCESS)),
                        )
                        .child(ui::caption(line).truncate()),
                )
            });

        let mut chrome = div()
            .h(px(52.0))
            .w_full()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .px_5()
            .gap_3()
            .border_b_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .child(identity);

        if let Some(status) = &self.status {
            let foreground = match status.tone {
                DocumentStatusTone::Success => tokens::SUCCESS,
                DocumentStatusTone::Error => tokens::DANGER,
            };
            chrome = chrome.child(
                div()
                    .text_xs()
                    .text_color(ui::color(foreground))
                    .child(status.message.clone()),
            );
        }
        let share_open = self.share_open;
        chrome = chrome.child(
            div()
                .id("share-handle")
                .role(gpui::Role::Button)
                .aria_label(ui::t("Share"))
                .aria_expanded(share_open)
                .flex_shrink_0()
                .px_3()
                .py(px(5.0))
                .rounded_md()
                .text_sm()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(ui::color(tokens::TEXT))
                .border_1()
                .border_color(ui::color(tokens::BORDER_STRONG))
                .when(share_open, |handle| {
                    handle.bg(ui::color(tokens::ROW_SELECTED))
                })
                .hover(|style| style.bg(ui::color(tokens::ROW_HOVER)))
                .cursor_pointer()
                .child(ui::t("Share"))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.share_open = !this.share_open;
                    this.menu_open = false;
                    cx.notify();
                })),
        );
        // The app menu: Settings, About, Save As, the World codes, Help and
        // Quit, on every platform.
        let menu_open = self.menu_open;
        chrome = chrome.child(
            app_menu::menu_button("world-menu", menu_open)
                .on_click(cx.listener(|this, _, window, cx| this.toggle_menu(window, cx))),
        );
        let share = share_open.then(|| self.share_list(cx));
        let backdrop = menu_open.then(|| {
            app_menu::backdrop::<Self>(
                |this, window, cx| {
                    if this.menu_open {
                        this.toggle_menu(window, cx);
                    }
                },
                cx,
            )
        });
        let menu = menu_open.then(|| self.app_menu(cx));
        match self.demo_cue.take() {
            Some(DemoTurn::Ending) => {
                self.status = None;
                self.show_demo_farewell(cx);
            }
            None => {}
        }

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            // World menu items dispatch to the frontmost window's root, so a
            // World window answers them and Home greys them out.
            .on_action(cx.listener(|this, _: &about::SaveWorldAs, _, cx| this.save_as(cx)))
            .on_action(cx.listener(|this, _: &about::ReloadWorld, _, cx| this.reload(cx)))
            .on_action(
                cx.listener(|this, _: &about::CloseWindow, window, cx| this.close(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &about::CopyWorldCode, _, cx| this.copy_world_code(cx)),
            )
            .on_action(
                cx.listener(|this, _: &about::SaveWorldCode, _, cx| this.save_world_code(cx)),
            )
            .on_action(cx.listener(|this, _: &about::InviteGuest, _, cx| this.invite_guest(cx)))
            .on_action(cx.listener(|this, _: &about::InviteFriend, _, cx| this.invite_friend(cx)))
            .on_action(cx.listener(|this, _: &about::ShowAsStrip, _, cx| {
                world_gpui::pointers::used(world_gpui::pointers::Pointer::Strip);
                let snapshot = this.projection.read(cx).snapshot().clone();
                strip_window::toggle(&this.document, snapshot, cx);
            }))
            .child(chrome)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .w_full()
                    .overflow_hidden()
                    .child(self.projection.clone()),
            )
            .children(share)
            .children(backdrop)
            .children(menu)
    }
}
