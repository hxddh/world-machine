#![forbid(unsafe_code)]
// A release build on Windows is a windowed program: no console window opens
// beside it. Debug builds keep the console for what the app prints.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(gui)]
mod about;
#[cfg(gui)]
mod build_info;
#[cfg(gui)]
mod diagnostics;
#[cfg(gui)]
mod included_packs;
#[cfg(gui)]
mod observer;
#[cfg(gui)]
mod saving;
#[cfg(gui)]
mod settings;
#[cfg(gui)]
mod sharing;
#[cfg(gui)]
mod strip_window;
#[cfg(gui)]
mod system_open;
#[cfg(gui)]
mod updates;
#[cfg(gui)]
/// A light-palette colour adapted to the current appearance.
pub(crate) fn theme_rgb(hex: u32) -> gpui::Rgba {
    gpui::rgb(world_theme::adapt(hex))
}

/// Re-renders every window when the system switches between light and dark;
/// each window root then records the new appearance before it draws.
#[cfg(gui)]
pub(crate) fn watch_appearance(window: &mut Window) {
    window
        .observe_window_appearance(|_, cx| cx.refresh_windows())
        .detach();
}
#[cfg(gui)]
mod library_setup;
#[cfg(gui)]
mod window_geometry;
#[cfg(gui)]
mod world_files;
#[cfg(gui)]
mod world_voice;
#[cfg(gui)]
use library_setup::*;
#[cfg(gui)]
use window_geometry::*;
#[cfg(gui)]
use world_files::*;

#[cfg(gui)]
use gpui::{
    div, point, prelude::*, px, size, App, AppContext, Bounds, Context, Entity, Global,
    IntoElement, PathPromptOptions, Render, SharedString, Styled, Window, WindowBounds,
    WindowOptions,
};
#[cfg(gui)]
use std::cell::RefCell;
#[cfg(gui)]
use std::env;
#[cfg(gui)]
use std::path::{Path, PathBuf};
#[cfg(gui)]
use std::process;
#[cfg(gui)]
use std::rc::Rc;
#[cfg(gui)]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
#[cfg(gui)]
use std::time::{Duration, SystemTime, UNIX_EPOCH};
#[cfg(gui)]
use world_gpui::text_input::{self as text_field, TextInput};
#[cfg(gui)]
use world_gpui::ui;
#[cfg(gui)]
use world_library::{
    DurableWorldSession, LibraryError, UnreadableWorldFile, WorldDocumentId, WorldDocumentSummary,
    WorldLibrary, LEGACY_WORLD_DOCUMENT_SUFFIX, WORLD_DOCUMENT_SUFFIX,
};
#[cfg(gui)]
use world_machine_desktop::ambience;
#[cfg(gui)]
use world_machine_desktop::demo;
#[cfg(gui)]
use world_machine_desktop::window_state::{self, StoredWindowBounds};
#[cfg(gui)]
use world_pack_bundle::PACK_BUNDLE_SUFFIX;
#[cfg(gui)]
use world_pack_catalog::{
    InstalledPack, PackAvailability, PackCatalog, PackInstallPreview, PackRefresh,
};
#[cfg(gui)]
use world_persistence::WorldPackRef;
#[cfg(gui)]
use world_theme::tokens;

#[cfg(gui)]
static LIBRARY_CHANGE_REVISION: AtomicU64 = AtomicU64::new(0);

#[cfg(gui)]
pub(crate) fn mark_library_changed() {
    LIBRARY_CHANGE_REVISION.fetch_add(1, Ordering::Relaxed);
}

#[cfg(gui)]
fn library_change_revision() -> u64 {
    // A World's file written away from its turn changes the library too.
    LIBRARY_CHANGE_REVISION.load(Ordering::Relaxed) + world_library::files_written()
}

#[cfg(gui)]
struct SharedDocumentState {
    session: DurableWorldSession,
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
}

#[cfg(gui)]
type SharedDocument = Rc<RefCell<SharedDocumentState>>;

#[cfg(gui)]
struct HostProjectionController {
    document: SharedDocument,
    /// What the World held when last seen, for the sound to hear a letter
    /// that a turn brought.
    tally: std::cell::Cell<Option<ambience::Tally>>,
    /// Set when the demo held back the day after its last: the window then
    /// shows the ending card (see `demo`).
    demo_ending: Rc<std::cell::Cell<bool>>,
}

#[cfg(gui)]
impl HostProjectionController {
    fn new(document: SharedDocument, demo_ending: Rc<std::cell::Cell<bool>>) -> Self {
        Self {
            document,
            tally: std::cell::Cell::new(None),
            demo_ending,
        }
    }
}

/// What the demo does with `intent` in this World: the day after its last
/// is answered with the farewell instead, and a World of a Pack it does
/// not offer is not played at all (see `demo::gate`, which fails closed).
#[cfg(gui)]
fn demo_holds(session: &DurableWorldSession, intent: &world_gpui::ProjectionIntent) -> demo::Gate {
    if !demo::ENABLED {
        return demo::Gate::Open;
    }
    let pack = &session.pack().id;
    let snapshot = session.snapshot();
    let passes_time = match intent {
        world_gpui::ProjectionIntent::InvokeCommand(command) => {
            demo::passes_time(&snapshot, command)
        }
        // Choosing, branching or talking moves no time, but a World the
        // demo does not offer is not played at all.
        _ if !demo::offers_pack(pack) => false,
        _ => return demo::Gate::Open,
    };
    let day = snapshot
        .calendar
        .as_ref()
        .map(|calendar| demo::day_of(snapshot.world_time, calendar.length));
    demo::gate(pack, day, passes_time)
}

/// What a World shows that its sound listens for.
#[cfg(gui)]
fn tally(snapshot: &world_gpui::ProjectionSnapshot) -> ambience::Tally {
    ambience::Tally {
        built: snapshot.canvas.marks.len(),
        keepsakes: snapshot.keepsakes.len(),
        letters: snapshot.letters.len(),
    }
}

#[cfg(gui)]
impl world_gpui::ProjectionController for HostProjectionController {
    fn snapshot(&self) -> world_gpui::ProjectionSnapshot {
        let snapshot = world_gpui::i18n::localize(self.document.borrow().session.snapshot());
        self.tally.set(Some(tally(&snapshot)));
        snapshot
    }

    fn cue(&mut self, cue: world_gpui::Cue) {
        ambience::player::cue(cue);
    }

    fn story(&mut self, request: world_gpui::StoryRequest) -> Option<world_gpui::StoryPage> {
        let page = self.document.borrow().session.story(request);
        match page {
            Ok(page) => page.map(world_gpui::i18n::localize_story),
            Err(error) => {
                diagnostics::error(format!("story: {error}"));
                None
            }
        }
    }

    /// With the World voice on, the model is asked here rather than inside
    /// the Pack, so the window can wait for it without freezing: the World
    /// gives the prompt, the model is asked off the window's thread, and
    /// the words are said with its response.
    fn listen(
        &mut self,
        to: world_gpui::SelectionId,
        words: &str,
    ) -> Option<world_gpui::Listening> {
        if !world_voice::voice_on() {
            return None;
        }
        // The World says what it knows, as data; the app builds the
        // prompt itself, so no World chooses what the player's key asks.
        let hearing = self
            .document
            .borrow()
            .session
            .voice_hearing(to, words)
            .ok()??;
        Some(Box::new(move || world_voice::ask_model(&hearing)))
    }

    fn handle(
        &mut self,
        intent: world_gpui::ProjectionIntent,
    ) -> Result<world_gpui::ProjectionSnapshot, String> {
        let mut document = self.document.borrow_mut();
        match demo_holds(&document.session, &intent) {
            demo::Gate::Open => {}
            demo::Gate::Ending => {
                self.demo_ending.set(true);
                return Ok(world_gpui::i18n::localize(document.session.snapshot()));
            }
            demo::Gate::NotOffered => return Err(ui::t(demo::NOT_OFFERED).to_string()),
        }
        let registry = Arc::clone(&document.registry);
        let library = Arc::clone(&document.library);
        let is_library_world = document.session.document_id().is_some();
        let result = document
            .session
            .handle(intent, &registry, &library)
            .map_err(|error| error.to_string());
        if result.is_ok() && is_library_world {
            mark_library_changed();
        }
        let result = result.map(world_gpui::i18n::localize);
        // A letter that came with the turn is heard just after its answer.
        if let Ok(snapshot) = &result {
            let after = tally(snapshot);
            if let Some(before) = self.tally.replace(Some(after)) {
                if ambience::letter_came(before, after) {
                    ambience::player::act_later(ambience::Act::Letter, 0.6);
                }
            }
        }
        result
    }
}

#[cfg(gui)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocumentStatusTone {
    Success,
    Error,
}

#[cfg(gui)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct DocumentStatus {
    message: String,
    tone: DocumentStatusTone,
}

#[cfg(gui)]
impl DocumentStatus {
    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: DocumentStatusTone::Success,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        let message = message.into();
        diagnostics::error(format!("document: {message}"));
        Self {
            message,
            tone: DocumentStatusTone::Error,
        }
    }
}

#[cfg(gui)]
struct WorldDocumentView {
    /// The durable identity of the World's file. Stays visible so a World can
    /// always be matched to the file it lives in.
    document_label: String,
    /// What this World is called: the name its owner gave it on Home, or the
    /// durable identity when it has none.
    document_name: String,
    document: SharedDocument,
    projection: Entity<world_gpui::ProjectionView>,
    status: Option<DocumentStatus>,
    /// When, in Unix seconds, this World next moves on its own; read once
    /// when it opens, since it only moves between visits.
    next_move_at: Option<u64>,
    /// Whether the Share list under the title bar is open.
    share_open: bool,
    /// Whether the demo's ending card is up (set by the controller when the
    /// day after the demo's last is asked for; always false in the full app).
    demo_ending: Rc<std::cell::Cell<bool>>,
    /// Set when the player chose to close this window although its World's
    /// latest turns could not be written (see [`Self::ready_to_close`]).
    closing_anyway: bool,
}

/// A World's own view in its window: the scene with no title bar of its
/// own (the window has one), and a handle that shows it as a strip.
#[cfg(gui)]
fn world_view(controller: HostProjectionController) -> world_gpui::ProjectionView {
    world_gpui::ProjectionView::controlled(controller)
        .without_header()
        .with_strip(|window, cx| window.dispatch_action(Box::new(about::ShowAsStrip), cx))
}

/// `view`'s window asks it before closing (see
/// [`WorldDocumentView::ready_to_close`]).
#[cfg(gui)]
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
    view
}

/// The World `id` as it is open in this app (in a window or a strip), if it
/// is: Home renames, exports and removes an open World through its session,
/// never behind its back.
#[cfg(gui)]
fn open_world_document(cx: &App, id: &WorldDocumentId) -> Option<SharedDocument> {
    open_world_where(cx, |session| session.document_id() == Some(id))
}

/// The open World whose session `is` says it is the one.
#[cfg(gui)]
fn open_world_where(cx: &App, is: impl Fn(&DurableWorldSession) -> bool) -> Option<SharedDocument> {
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
#[cfg(gui)]
fn refresh_world_windows(document: &SharedDocument, cx: &mut App) {
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
#[cfg(gui)]
fn focus_open_world(cx: &mut App, id: &WorldDocumentId) -> bool {
    let document = open_world_document(cx, id);
    focus_world(cx, document)
}

/// Brings the window of `document` to the front; `false` if there is none.
#[cfg(gui)]
fn focus_world(cx: &mut App, document: Option<SharedDocument>) -> bool {
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
#[cfg(gui)]
fn unsaved_worlds(cx: &App) -> Vec<(String, String)> {
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
#[cfg(gui)]
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

#[cfg(gui)]
impl WorldDocumentView {
    fn new(
        session: DurableWorldSession,
        registry: Arc<world_host::WorldRegistry>,
        library: Arc<WorldLibrary>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::with_document(
            Rc::new(RefCell::new(SharedDocumentState {
                session,
                registry,
                library,
            })),
            cx,
        )
    }

    /// A window onto a World that is already open elsewhere, such as in a
    /// strip along the edge of the screen.
    fn with_document(document: SharedDocument, cx: &mut Context<Self>) -> Self {
        let (document_label, document_name, next_move_at, read_only) = {
            let state = document.borrow();
            (
                state.session.display_name(),
                session_display_name(&state.session),
                observer::next_move_in(&state.session, &state.library)
                    .zip(unix_now())
                    .map(|(remaining, now)| now + remaining),
                // A World a newer World Machine saved is only looked at.
                state.session.read_only_reason(),
            )
        };
        let demo_ending = Rc::new(std::cell::Cell::new(false));
        let controller =
            HostProjectionController::new(Rc::clone(&document), Rc::clone(&demo_ending));
        let projection = cx.new(|_| world_view(controller));
        // The title bar reads the World's name and what it can do from the
        // page, so it redraws whenever the page does.
        cx.observe(&projection, |_, _, cx| cx.notify()).detach();
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
            status: read_only.map(DocumentStatus::error),
            next_move_at,
            share_open: false,
            demo_ending,
            closing_anyway: false,
        }
    }

    /// Asked before this window closes: the World's changes are written
    /// first, and if they cannot be, the window stays open and asks what to
    /// do (v0.26 let a failed last write go with the window, in silence).
    fn ready_to_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
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
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.ready_to_close(window, cx) {
            window.remove_window();
        }
    }

    /// Re-read what this World is called from the session, after anything
    /// that can change its file or its target.
    fn refresh_document_identity(&mut self) {
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

    fn reload(&mut self, cx: &mut Context<Self>) {
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

    fn save_as(&mut self, cx: &mut Context<Self>) {
        let semantic_title = self.document.borrow().session.snapshot().title;
        let suggested_name = suggested_world_file_name(&semantic_title, &self.document_name);
        let save_dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested_name));
        cx.spawn(async move |this, cx| {
            let destination = match save_dialog.await {
                Ok(Ok(Some(path))) => canonical_world_path(path),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(DocumentStatus::error(format!(
                            "Could not open Save As dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(DocumentStatus::error(format!(
                            "Save As dialog was interrupted: {error}"
                        )));
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
                        this.rebuild_projection(cx);
                        this.status = Some(DocumentStatus::success(format!(
                            "Saved As {} · {}",
                            this.document_name,
                            world_gpui::i18n::moment_label(&snapshot, snapshot.world_time)
                        )));
                    }
                    Err(error) => {
                        this.status =
                            Some(DocumentStatus::error(format!("Save As failed: {error}")));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn rebuild_projection(&mut self, cx: &mut Context<Self>) {
        let controller =
            HostProjectionController::new(Rc::clone(&self.document), Rc::clone(&self.demo_ending));
        self.projection = cx.new(|_| world_view(controller));
    }

    /// Share, under the title bar: the World's code, copied or saved, and
    /// a guest from another World; the same as the World menu's items.
    fn share_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    /// The demo's ending: the farewell over the World, at dusk, recapping
    /// the player's own moments from what the World recorded, with a
    /// resident's goodbye and a postcard to keep. It asks for nothing;
    /// "Stay a while" puts it away, and it comes back only when the next
    /// day is asked for again.
    fn show_demo_farewell(&mut self, cx: &mut Context<Self>) {
        self.demo_ending.set(false);
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
                more: Some((demo::ENDING_FULL_APP.into(), about_the_full_app())),
                stay: demo::ENDING_STAY.into(),
                hour: demo::FAREWELL_HOUR,
            });
            cx.notify();
        });
    }
}

/// What "About the full app" does from the demo's farewell.
#[cfg(gui)]
fn about_the_full_app() -> world_gpui::FarewellAction {
    std::rc::Rc::new(|_: &mut Window, cx: &mut gpui::App| cx.open_url(demo::FULL_APP_URL))
}

#[cfg(gui)]
impl Render for WorldDocumentView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        remember_window_geometry(window, RememberedWindow::World);
        // A World's name follows it as it changes ("A new World" becomes
        // "Ares Pocket Colony" when it is seeded), so read it every time.
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
        let keeps_going = self
            .next_move_at
            .filter(|_| moves_alone && !farewell)
            .zip(unix_now())
            .map(|(at, now)| keeps_going_line(at.saturating_sub(now), &unit));
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
                    cx.notify();
                })),
        );
        let share = share_open.then(|| self.share_list(cx));
        if self.demo_ending.get() {
            self.show_demo_farewell(cx);
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
    }
}

#[cfg(gui)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HomeStatusTone {
    Info,
    Success,
    Error,
}

#[cfg(gui)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct HomeStatus {
    message: String,
    tone: HomeStatusTone,
}

#[cfg(gui)]
impl HomeStatus {
    fn info(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: HomeStatusTone::Info,
        }
    }

    fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tone: HomeStatusTone::Success,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        let message = message.into();
        diagnostics::error(format!("home: {message}"));
        Self {
            message,
            tone: HomeStatusTone::Error,
        }
    }
}

#[cfg(gui)]
struct WorldMachineHome {
    registry: Arc<world_host::WorldRegistry>,
    library: Arc<WorldLibrary>,
    pack_catalog: Option<PackCatalog>,
    pack_catalog_path: PathBuf,
    documents: Vec<WorldDocumentSummary>,
    selected_world_pack: Option<String>,
    included_packs: Vec<included_packs::IncludedPack>,
    pending_pack_install: Option<PackInstallPreview>,
    pending_start_after_install: Option<WorldPackRef>,
    ready_pack_to_create: Option<WorldPackRef>,
    probing_packs: Vec<WorldPackRef>,
    status: Option<HomeStatus>,
    /// Installed-Pack management is hidden behind one line on Home until asked
    /// for; nothing in the ordinary path needs it.
    show_packs: bool,
    /// A newer stable release found at launch, shown as a banner until
    /// dismissed or downloaded.
    available_update: Option<updates::AvailableUpdate>,
    /// Files in the Worlds folder that name themselves Worlds but cannot be
    /// read. They are named on Home instead of hiding every other World.
    unreadable_documents: Vec<UnreadableWorldFile>,
    /// The World whose name is being typed, if any.
    renaming: Option<RenameDraft>,
    /// The World whose removal is waiting for a second click.
    pending_removal: Option<WorldDocumentId>,
    /// The World whose ⋯ menu is open on Home.
    card_menu: Option<WorldDocumentId>,
    /// How My Worlds is ordered, for this run of the app.
    world_sort: WorldSort,
    /// What was typed into Find a World.
    world_search: Entity<TextInput>,
    /// On a first launch Home steps aside once the first World opens, so a
    /// new player meets one window, not two.
    step_aside_for_first_world: bool,
}

/// A World name being typed on Home. Only one World is renamed at a time, so
/// the field lives here rather than one per card.
#[cfg(gui)]
struct RenameDraft {
    document: WorldDocumentId,
    input: Entity<TextInput>,
}

#[cfg(gui)]
impl WorldMachineHome {
    fn start_system_open_listener(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let mut observed_library_revision = library_change_revision();
            let mut ticks_since_geometry_flush = 0u32;
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(200))
                    .await;

                let Some(this) = this.upgrade() else {
                    return;
                };

                // Windows report where they are as they render; write any
                // change a few times a second rather than on every frame of a
                // drag.
                ticks_since_geometry_flush += 1;
                if ticks_since_geometry_flush >= WINDOW_GEOMETRY_FLUSH_TICKS {
                    ticks_since_geometry_flush = 0;
                    flush_window_geometry();
                }
                let paths = system_open::drain_paths();
                let revision = library_change_revision();
                let library_changed = revision != observed_library_revision;
                if library_changed {
                    observed_library_revision = revision;
                }
                if paths.is_empty() && !library_changed {
                    continue;
                }

                this.update(cx, |this, cx| {
                    for path in paths {
                        match path {
                            Ok(path) => this.open_external_path(path, cx),
                            Err(error) => {
                                this.status = Some(HomeStatus::error(format!(
                                    "Could not open World file: {error}"
                                )));
                            }
                        }
                    }
                    if library_changed {
                        if let Err(error) = this.refresh_documents() {
                            this.status = Some(error);
                        }
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn rebuild_registry(&mut self) -> Result<(), String> {
        let registry = build_registry(self.pack_catalog.as_ref())?;
        self.registry = Arc::new(registry);
        Ok(())
    }

    fn review_pack_path(
        &mut self,
        source: PathBuf,
        expected_pack: Option<WorldPackRef>,
        start_after_install: bool,
        cx: &mut Context<Self>,
    ) {
        self.pending_start_after_install = None;
        if self.pack_catalog.is_none() {
            match PackCatalog::open(&self.pack_catalog_path) {
                Ok(catalog) => self.pack_catalog = Some(catalog),
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not open Installed Packs catalog {}: {error}",
                        self.pack_catalog_path.display()
                    )));
                    cx.notify();
                    return;
                }
            }
        }

        let catalog = self.pack_catalog.as_ref().unwrap();
        match catalog.inspect_install(&source) {
            Ok(preview) => {
                if expected_pack
                    .as_ref()
                    .is_some_and(|expected| preview.pack() != expected)
                {
                    let expected = expected_pack.unwrap();
                    self.pending_pack_install = None;
                    self.status = Some(HomeStatus::error(format!(
                        "Included Pack identity mismatch: expected {} @ {}, found {} @ {}",
                        expected.id,
                        expected.version,
                        preview.pack().id,
                        preview.pack().version
                    )));
                    cx.notify();
                    return;
                }
                self.status = Some(HomeStatus::info(format!(
                    "Review {} @ {} before trusting its executable bytes",
                    preview.pack().id,
                    preview.pack().version
                )));
                self.pending_start_after_install =
                    start_after_install.then(|| preview.pack().clone());
                self.pending_pack_install = Some(preview);
            }
            Err(error) => {
                self.pending_pack_install = None;
                self.status = Some(HomeStatus::error(format!(
                    "Could not inspect {}: {error}",
                    source.display()
                )));
            }
        }
        cx.notify();
    }

    fn review_included_pack(
        &mut self,
        pack: included_packs::IncludedPack,
        start_after_install: bool,
        cx: &mut Context<Self>,
    ) {
        self.review_pack_path(pack.path, Some(pack.pack), start_after_install, cx);
    }

    /// Install, probe, and activate the World Packs shipped inside this app
    /// bundle so a fresh install reaches a living World without a review
    /// dialog. The bundle is one code-signed unit, so these Packs carry the
    /// same trust as the app binary; the catalog still pins their exact
    /// content on install and the durable probe still runs before activation.
    /// User-supplied `.worldpack` files keep the explicit review flow.
    /// Packs that are already in the catalog, enabled or not, keep that state;
    /// only their program is brought up to the one this app ships.
    /// One background request to the Releases API; a newer stable version
    /// becomes a banner on Home. Silent on failure, off with
    /// WORLD_MACHINE_NO_UPDATE_CHECK=1.
    fn start_update_check(&mut self, cx: &mut Context<Self>) {
        if !updates::enabled() {
            return;
        }
        let task = cx
            .background_executor()
            .spawn(async move { updates::check() });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(update) = result {
                    diagnostics::info(format!("update available: {}", update.version));
                    this.available_update = Some(update);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn update_banner(
        &self,
        update: updates::AvailableUpdate,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let url = update.url.clone();
        div()
            .id("update-available")
            .w_full()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .text_sm()
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(format!(
                        "World Machine {} is available. You have {}.",
                        update.version,
                        build_info::APP_VERSION
                    )),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap_2()
                    .child(
                        ui::button("download-update", "Download", ui::ButtonKind::Secondary)
                            .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url))),
                    )
                    .child(
                        ui::button("dismiss-update", "Later", ui::ButtonKind::Secondary).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.available_update = None;
                                cx.notify();
                            }),
                        ),
                    ),
            )
    }

    fn activate_included_packs(&mut self, cx: &mut Context<Self>) {
        let packs = self.included_packs.clone();
        for pack in packs {
            if self.included_pack_is_installed(&pack.pack) {
                self.refresh_included_pack(&pack);
                continue;
            }
            let Some(catalog) = self.pack_catalog.as_mut() else {
                return;
            };
            let preview = match catalog.inspect_install(&pack.path) {
                Ok(preview) => preview,
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not prepare {}: {error}",
                        pack.title
                    )));
                    continue;
                }
            };
            if preview.pack() != &pack.pack {
                self.status = Some(HomeStatus::error(format!(
                    "Could not prepare {}: bundle contains {} @ {}, expected {} @ {}",
                    pack.title,
                    preview.pack().id,
                    preview.pack().version,
                    pack.pack.id,
                    pack.pack.version
                )));
                continue;
            }
            match catalog.install_reviewed_pending_probe(&preview) {
                Ok(installed) => {
                    // A fresh install (no saved Worlds yet) opens straight
                    // into its first World once the featured Pack is ready;
                    // otherwise Home offers the Create handoff.
                    let create_now = pack.featured && self.documents.is_empty();
                    self.step_aside_for_first_world |= create_now;
                    self.start_pack_probe(
                        installed.pack,
                        true,
                        create_now,
                        pack.featured && !create_now,
                        cx,
                    );
                    self.status = Some(HomeStatus::info(format!(
                        "Preparing {} for its first launch…",
                        pack.title
                    )));
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not prepare {}: {error}",
                        pack.title
                    )));
                }
            }
        }
        cx.notify();
    }

    /// An included Pack keeps its version when its program is fixed (the
    /// Worlds made with it are stamped with that version), so a catalog that
    /// installed it from an earlier copy of the app would keep the earlier
    /// program. Where the program or manifest this app ships differs, it
    /// replaces the installed one, which stays enabled and active as it was.
    fn refresh_included_pack(&mut self, pack: &included_packs::IncludedPack) {
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        match catalog.refresh_bundle(&pack.path) {
            Ok(PackRefresh::Replaced(installed)) => {
                diagnostics::info(format!(
                    "included pack {} @ {} replaced with the program this app ships",
                    installed.pack.id, installed.pack.version
                ));
                if let Err(error) = self.rebuild_registry() {
                    self.status = Some(HomeStatus::error(format!(
                        "{} was updated, but Registry rebuild failed: {error}",
                        pack.title
                    )));
                }
            }
            Ok(PackRefresh::Unchanged | PackRefresh::NotInstalled) => {}
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not update {}: {error}",
                    pack.title
                )));
            }
        }
    }

    fn install_pack(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Install World Pack".into()),
        });
        cx.spawn(async move |this, cx| {
            let source = match picker.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Could not open Install Pack dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Install Pack dialog was interrupted: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(source) = source else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                this.review_pack_path(source, None, false, cx)
            });
        })
        .detach();
    }

    fn confirm_pack_install(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.pending_pack_install.clone() else {
            return;
        };
        let start_after_install =
            start_after_install_matches(self.pending_start_after_install.as_ref(), preview.pack());
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        let result = catalog.install_reviewed_pending_probe(&preview);
        self.pending_pack_install = None;
        self.pending_start_after_install = None;
        self.ready_pack_to_create = None;
        match result {
            Ok(installed) => {
                self.start_pack_probe(installed.pack, true, start_after_install, true, cx);
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Pack was not installed. Re-open it to review current content: {error}"
                )));
                cx.notify();
            }
        }
    }

    fn is_pack_probing(&self, pack: &WorldPackRef) -> bool {
        self.probing_packs.iter().any(|candidate| candidate == pack)
    }

    fn start_pack_probe(
        &mut self,
        pack: WorldPackRef,
        activate_on_success: bool,
        create_on_success: bool,
        offer_create_on_success: bool,
        cx: &mut Context<Self>,
    ) {
        if self.is_pack_probing(&pack) {
            return;
        }
        let Some(catalog) = self.pack_catalog.clone() else {
            return;
        };
        self.probing_packs.push(pack.clone());
        self.status = Some(HomeStatus::info(if create_on_success {
            "Testing this World before first launch…".into()
        } else if let Some(title) = self.included_pack_title(&pack) {
            format!("Preparing {title} for its first launch…")
        } else {
            format!(
                "Testing trusted Pack {} @ {} · Create → Archive → fresh-process Open…",
                pack.id, pack.version
            )
        }));
        cx.notify();

        let probe_pack = pack.clone();
        let task = cx
            .background_executor()
            .spawn(async move { catalog.probe(&probe_pack) });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.probing_packs.retain(|candidate| candidate != &pack);
                match result {
                    Ok(probe) => {
                        let transition = this
                            .pack_catalog
                            .as_mut()
                            .ok_or_else(|| "Installed Packs catalog is unavailable".to_string())
                            .and_then(|catalog| {
                                catalog
                                    .set_enabled(&pack, true)
                                    .map_err(|error| error.to_string())?;
                                if activate_on_success {
                                    catalog.activate(&pack).map_err(|error| error.to_string())?;
                                }
                                Ok(())
                            });
                        match transition {
                            Ok(()) => match this.rebuild_registry() {
                                Ok(()) => {
                                    if activate_on_success && create_on_success {
                                        this.ready_pack_to_create = None;
                                        this.create_world(pack.id.clone(), cx);
                                        return;
                                    }
                                    if activate_on_success && offer_create_on_success {
                                        this.ready_pack_to_create = Some(pack.clone());
                                    }
                                    diagnostics::info(format!(
                                        "pack {} @ {} passed its durable probe · World time {} → {}",
                                        pack.id,
                                        pack.version,
                                        probe.created_world_time,
                                        probe.reopened_world_time
                                    ));
                                    // An included Pack getting ready in the background
                                    // is not news; the log keeps the probe's numbers.
                                    // A Pack the person installed themselves is.
                                    this.status = if this.included_pack_title(&pack).is_some() {
                                        None
                                    } else {
                                        let title = this
                                            .registry
                                            .descriptor_for(&pack)
                                            .map(|descriptor| descriptor.title.clone())
                                            .unwrap_or_else(|| pack.id.clone());
                                        Some(HomeStatus::success(format!(
                                            "{title} is installed and ready."
                                        )))
                                    };
                                }
                                Err(error) => {
                                    this.status = Some(HomeStatus::error(format!(
                                        "Pack {} @ {} passed its durable probe, but Registry rebuild failed: {error}",
                                        pack.id, pack.version
                                    )));
                                }
                            },
                            Err(error) => {
                                this.status = Some(HomeStatus::error(format!(
                                    "Pack {} @ {} passed its durable probe, but could not be enabled: {error}",
                                    pack.id, pack.version
                                )));
                            }
                        }
                    }
                    Err(error) => {
                        if this.ready_pack_to_create.as_ref() == Some(&pack) {
                            this.ready_pack_to_create = None;
                        }
                        let _ = this.rebuild_registry();
                        this.status = Some(HomeStatus::error(format!(
                            "Installed and trusted {} @ {}, but its durable activation probe failed. The Pack remains disabled: {error}",
                            pack.id, pack.version
                        )));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn included_pack_title(&self, pack: &WorldPackRef) -> Option<&'static str> {
        self.included_packs
            .iter()
            .find(|included| &included.pack == pack)
            .map(|included| included.title)
    }

    fn ready_pack_descriptor(&self) -> Option<world_host::WorldDescriptor> {
        let pack = self.ready_pack_to_create.as_ref()?;
        // "Start your first World" is untrue for someone who already has one.
        if self
            .documents
            .iter()
            .any(|document| document.pack.id == pack.id)
        {
            return None;
        }
        let catalog = self.pack_catalog.as_ref()?;
        let installed = catalog.entry(pack)?;
        if !installed.enabled || !installed.active {
            return None;
        }
        if !matches!(catalog.availability(pack), PackAvailability::Ready) {
            return None;
        }
        self.registry.descriptor_for(pack).cloned()
    }

    fn dismiss_ready_pack(&mut self, cx: &mut Context<Self>) {
        self.ready_pack_to_create = None;
        cx.notify();
    }

    fn cancel_pack_install(&mut self, cx: &mut Context<Self>) {
        self.pending_pack_install = None;
        self.pending_start_after_install = None;
        self.status = Some(HomeStatus::info(
            "Pack installation cancelled; no external code was installed.",
        ));
        cx.notify();
    }

    fn activate_pack(&mut self, pack: WorldPackRef, cx: &mut Context<Self>) {
        if self
            .ready_pack_to_create
            .as_ref()
            .is_some_and(|ready| ready.id == pack.id && ready != &pack)
        {
            self.ready_pack_to_create = None;
        }
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        match catalog.activate(&pack) {
            Ok(()) => match self.rebuild_registry() {
                Ok(()) => {
                    self.status = Some(HomeStatus::success(format!(
                        "Activated {} @ {} for new Worlds",
                        pack.id, pack.version
                    )))
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Changed active Pack to {} @ {}, but Registry rebuild failed: {error}",
                        pack.id, pack.version
                    )))
                }
            },
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not activate {} @ {}: {error}",
                    pack.id, pack.version
                )))
            }
        }
        cx.notify();
    }

    fn set_pack_enabled(&mut self, pack: WorldPackRef, enabled: bool, cx: &mut Context<Self>) {
        if !enabled && self.ready_pack_to_create.as_ref() == Some(&pack) {
            self.ready_pack_to_create = None;
        }
        let Some(catalog) = self.pack_catalog.as_mut() else {
            return;
        };
        match catalog.set_enabled(&pack, enabled) {
            Ok(()) => match self.rebuild_registry() {
                Ok(()) => {
                    self.status = Some(HomeStatus::success(format!(
                        "{} {} @ {}",
                        if enabled { "Enabled" } else { "Disabled" },
                        pack.id,
                        pack.version
                    )))
                }
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Updated {} @ {}, but Registry rebuild failed: {error}",
                        pack.id, pack.version
                    )))
                }
            },
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not {} {} @ {}: {error}",
                    if enabled { "enable" } else { "disable" },
                    pack.id,
                    pack.version
                )))
            }
        }
        cx.notify();
    }

    fn missing_pack_message(&self, pack: &WorldPackRef) -> String {
        let availability = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.availability(pack))
            .unwrap_or(PackAvailability::NotInstalled);
        match availability {
            PackAvailability::Ready => format!(
                "Could not open World: {} @ {} should be available, but is not registered",
                pack.id, pack.version
            ),
            PackAvailability::Disabled => format!(
                "Could not open World: {} @ {} is installed but disabled. Use Test & Enable under Installed Packs.",
                pack.id, pack.version
            ),
            PackAvailability::Invalid { reason } => format!(
                "Could not open World: {} @ {} is installed but no longer matches its approved content: {reason}",
                pack.id, pack.version
            ),
            PackAvailability::MissingVersion { installed_versions } => format!(
                "Could not open World: it requires {} @ {}, but installed versions are {}. Install that exact Pack version.",
                pack.id,
                pack.version,
                installed_versions.join(", ")
            ),
            PackAvailability::NotInstalled => format!(
                "Could not open World: it requires {} @ {}, which is not installed. Use Install Pack… to add that exact version.",
                pack.id, pack.version
            ),
        }
    }

    fn refresh_documents(&mut self) -> Result<usize, HomeStatus> {
        let listing = self
            .library
            .listing()
            .map_err(|error| HomeStatus::error(format!("Could not read World Library: {error}")))?;
        let count = listing.documents.len();
        self.documents = listing.documents;
        self.unreadable_documents = listing.unreadable;
        report_unreadable_documents(&self.unreadable_documents);
        // A World that is gone from the Library cannot still be mid-rename or
        // mid-removal on a card.
        let documents = &self.documents;
        let renaming_is_gone = self.renaming.as_ref().is_some_and(|draft| {
            !documents
                .iter()
                .any(|document| document.id == draft.document)
        });
        let removal_is_gone = self
            .pending_removal
            .as_ref()
            .is_some_and(|pending| !documents.iter().any(|document| &document.id == pending));
        if renaming_is_gone {
            self.renaming = None;
        }
        if removal_is_gone {
            self.pending_removal = None;
        }
        if !world_pack_filter_is_available(&self.documents, self.selected_world_pack.as_deref()) {
            self.selected_world_pack = None;
        }
        Ok(count)
    }

    fn refresh_from_menu(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match self.refresh_documents() {
            Ok(count) => HomeStatus::success(format!("My Worlds · {count} World(s)")),
            Err(status) => status,
        });
        cx.notify();
    }

    fn sync_documents_after_mutation(&mut self) -> Option<HomeStatus> {
        self.refresh_documents().err()
    }

    /// Opens a World window.
    fn open_session(
        &mut self,
        mut session: DurableWorldSession,
        title: String,
        cx: &mut Context<Self>,
    ) {
        let is_library_world = session.document_id().is_some();
        let catch_up = observer::catch_up(&mut session, &self.registry, &self.library);
        let unit_after_catch_up = session
            .snapshot()
            .calendar
            .map(|calendar| calendar.unit.to_lowercase())
            .unwrap_or_else(|| "day".into());
        let sync_error = if is_library_world && matches!(&catch_up, Ok(Some(_))) {
            self.refresh_documents().err()
        } else {
            None
        };
        let registry = Arc::clone(&self.registry);
        let library = Arc::clone(&self.library);
        let bounds = remembered_window_bounds(RememberedWindow::World, cx)
            .unwrap_or_else(|| Bounds::centered(None, size(px(1100.0), px(900.0)), cx));
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                watch_appearance(window);
                let view = cx.new(|cx| WorldDocumentView::new(session, registry, library, cx));
                ask_before_closing(view, window, cx)
            },
        );

        // A window opening is its own confirmation; Home only speaks up when
        // there is something the window does not already say.
        self.status = match opened {
            Ok(_) => match catch_up {
                Ok(Some(outcome)) => Some(HomeStatus::success(format!(
                    "{title} lived {} while you were away",
                    count_of(outcome.periods, &unit_after_catch_up)
                ))),
                Ok(None) => None,
                Err(error) => Some(HomeStatus::info(format!(
                    "Opened {title}, but the time you were away could not be caught up: {error}"
                ))),
            },
            Err(error) => Some(HomeStatus::error(format!(
                "Could not open {title}: {error}"
            ))),
        };
        if let Some(status) = sync_error {
            self.status = Some(status);
        }
        cx.notify();
    }

    fn create_world(&mut self, pack_id: String, cx: &mut Context<Self>) {
        // A World of this kind that was opened but never begun is picked up
        // where it was left rather than joined by another empty one.
        if let Some(unbegun) = self
            .documents
            .iter()
            .find(|document| document.pack.id == pack_id && !has_begun(document))
            .map(|document| document.id.clone())
        {
            self.open_document(unbegun, cx);
            self.step_aside_if_first_world(cx);
            return;
        }
        let title = self
            .registry
            .descriptor(&pack_id)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| pack_id.clone());
        let document_id = match new_document_id(&pack_id, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not create {title}: {error}"
                )));
                cx.notify();
                return;
            }
        };
        let session =
            match DurableWorldSession::create(document_id, &pack_id, &self.registry, &self.library)
            {
                Ok(session) => session,
                Err(error) => {
                    self.status = Some(HomeStatus::error(format!(
                        "Could not create {title}: {error}"
                    )));
                    cx.notify();
                    return;
                }
            };
        let sync_error = self.sync_documents_after_mutation();
        if self
            .ready_pack_to_create
            .as_ref()
            .is_some_and(|ready| ready.id == pack_id)
        {
            self.ready_pack_to_create = None;
        }
        self.open_session(session, title, cx);
        self.step_aside_if_first_world(cx);
        if let Some(status) = sync_error {
            self.status = Some(status);
            cx.notify();
        }
    }

    /// Close Home once the first World of a first launch is open; it comes
    /// back when that World's window closes.
    fn step_aside_if_first_world(&mut self, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.step_aside_for_first_world) {
            return;
        }
        cx.defer(|cx| {
            let world_open = cx
                .windows()
                .iter()
                .any(|window| window.downcast::<WorldDocumentView>().is_some());
            if !world_open {
                return;
            }
            for window in cx.windows() {
                if let Some(home) = window.downcast::<WorldMachineHome>() {
                    let _ = home.update(cx, |_, window, _| window.remove_window());
                }
            }
        });
    }

    fn open_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        // One window, and one writer, per World: an open one comes forward.
        if focus_open_world(cx, &document_id) {
            return;
        }
        let summary = self
            .documents
            .iter()
            .find(|document| document.id == document_id);
        if let Some(document) = summary {
            if self.registry.descriptor_for(&document.pack).is_none() {
                self.status = Some(HomeStatus::error(self.missing_pack_message(&document.pack)));
                cx.notify();
                return;
            }
        }
        let title = summary
            .and_then(|document| self.registry.descriptor_for(&document.pack))
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document_id.to_string());
        let session = match DurableWorldSession::open(document_id, &self.registry, &self.library) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not open {title}: {error}"
                )));
                cx.notify();
                return;
            }
        };
        self.open_session(session, title, cx);
    }

    fn open_external_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if sharing::is_world_code_file(&source) {
            self.visit_path(&source, cx);
            return;
        }
        if is_world_pack_file(&source) {
            self.review_pack_path(source, None, false, cx);
            return;
        }
        if let Some(document_id) = library_document_id_for_path(&source, &self.library) {
            self.open_document(document_id, cx);
            return;
        }
        if !is_world_file(&source) {
            self.status = Some(HomeStatus::error(format!(
                "Could not open {}: choose a {} document or {} Pack",
                source.display(),
                WORLD_DOCUMENT_SUFFIX,
                PACK_BUNDLE_SUFFIX
            )));
            cx.notify();
            return;
        }
        // A World file already open here comes forward rather than opening
        // twice (it has one writer).
        let open = open_world_where(cx, |session| session.file_path() == Some(source.as_path()));
        if focus_world(cx, open) {
            return;
        }
        let session = match DurableWorldSession::open_file(source.clone(), &self.registry) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not open {}: {error}",
                    source.display()
                )));
                cx.notify();
                return;
            }
        };
        let pack = session.pack();
        let title = self
            .registry
            .descriptor_for(&pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        self.open_session(session, title, cx);
    }

    fn import_path(&mut self, source: PathBuf, cx: &mut Context<Self>) {
        if !is_world_file(&source) {
            self.status = Some(HomeStatus::error(format!(
                "Could not import {}: choose a {} file",
                source.display(),
                WORLD_DOCUMENT_SUFFIX
            )));
            cx.notify();
            return;
        }
        let document_id = match imported_document_id(&source, &self.library) {
            Ok(id) => id,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not import {}: {error}",
                    source.display()
                )));
                cx.notify();
                return;
            }
        };
        let session = match DurableWorldSession::import_file(
            document_id,
            &source,
            &self.registry,
            &self.library,
        ) {
            Ok(session) => session,
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not import {}: {error}",
                    source.display()
                )));
                cx.notify();
                return;
            }
        };
        let pack = session.pack();
        let title = self
            .registry
            .descriptor_for(&pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or(pack.id);
        let sync_error = self.sync_documents_after_mutation();
        self.open_session(session, title, cx);
        if let Some(status) = sync_error {
            self.status = Some(status);
            cx.notify();
        }
    }

    fn import_world(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import World".into()),
        });
        cx.spawn(async move |this, cx| {
            let source = match picker.await {
                Ok(Ok(Some(mut paths))) => paths.pop(),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Could not open Import dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Import dialog was interrupted: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
            };
            let Some(source) = source else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.import_path(source, cx));
        })
        .detach();
    }

    fn export_document(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        let semantic_title = self
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .and_then(|document| document.display_title.as_deref())
            .unwrap_or_default();
        let suggested_name = suggested_world_file_name(semantic_title, document_id.as_str());
        let save_dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested_name));
        cx.spawn(async move |this, cx| {
            let destination = match save_dialog.await {
                Ok(Ok(Some(path))) => canonical_world_path(path),
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Could not open Export dialog: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.status = Some(HomeStatus::error(format!(
                            "Export dialog was interrupted: {error}"
                        )));
                        cx.notify();
                    });
                    return;
                }
            };

            let _ = this.update(cx, |this, cx| {
                // An open World is exported through its session, its latest
                // turns written first.
                let exported = match open_world_document(cx, &document_id) {
                    Some(document) => match document.try_borrow() {
                        Ok(document) => document
                            .session
                            .export_file(&destination, &document.library),
                        Err(_) => Err(LibraryError::InUse(this.library.path(&document_id))),
                    },
                    None => this.library.export_file(&document_id, &destination),
                };
                match exported {
                    Ok(()) => {
                        this.status = Some(HomeStatus::success(format!(
                            "Exported {} to {}",
                            document_id,
                            destination.display()
                        )));
                    }
                    Err(error) => {
                        this.status = Some(HomeStatus::error(format!(
                            "Could not export {document_id}: {error}"
                        )));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn set_world_pack_filter(&mut self, pack_id: Option<String>, cx: &mut Context<Self>) {
        self.selected_world_pack = pack_id;
        cx.notify();
    }

    fn pack_filter_title(&self, pack_id: &str) -> String {
        self.registry
            .descriptor(pack_id)
            .map(|descriptor| descriptor.title.clone())
            .or_else(|| {
                self.documents
                    .iter()
                    .find(|document| document.pack.id == pack_id)
                    .and_then(|document| self.registry.descriptor_for(&document.pack))
                    .map(|descriptor| descriptor.title.clone())
            })
            .unwrap_or_else(|| pack_id.to_owned())
    }

    /// Find a World, and choose whether the list reads most-recent-first or
    /// A to Z. Shown only once there are enough Worlds for the list to be hard
    /// to scan.
    fn world_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut order = div().flex_shrink_0().flex().items_center().gap_2().child(
            div()
                .text_xs()
                .text_color(ui::color(tokens::TEXT_SECONDARY))
                .child(ui::t("Order")),
        );
        for sort in [WorldSort::Recent, WorldSort::Name] {
            let selected = self.world_sort == sort;
            let (border, background, text) = if selected {
                (0x6f86b0, 0xe9eef7, 0x314b72)
            } else {
                (0xd9d9d3, 0xffffff, 0x666666)
            };
            order = order.child(
                div()
                    .id(SharedString::from(format!(
                        "world-sort-{}",
                        sort.label().to_lowercase()
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(border))
                    .bg(crate::theme_rgb(background))
                    .text_color(crate::theme_rgb(text))
                    .text_xs()
                    .child(sort.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.world_sort = sort;
                        cx.notify();
                    })),
            );
        }

        div()
            .w_full()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .child(self.world_search.clone()),
            )
            .child(order)
    }

    fn document_pack_title(&self, document_id: &WorldDocumentId) -> String {
        self.documents
            .iter()
            .find(|document| &document.id == document_id)
            .map(|document| {
                self.registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone())
            })
            .unwrap_or_else(|| document_id.to_string())
    }

    /// Open the name field on a World card. A World that already has a name
    /// opens on that name; an unnamed one opens empty and shows its World
    /// Pack's title as the placeholder.
    fn begin_rename(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        text_field::bind_keys(cx);
        let current = self
            .documents
            .iter()
            .find(|document| document.id == document_id)
            .and_then(|document| document.display_title.clone())
            .unwrap_or_default();
        let placeholder = rename_placeholder(&self.document_pack_title(&document_id));
        let input = cx.new(|cx| TextInput::new(placeholder, cx).with_text(current));
        self.pending_removal = None;
        self.renaming = Some(RenameDraft {
            document: document_id,
            input,
        });
        cx.notify();
    }

    fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming = None;
        cx.notify();
    }

    fn commit_rename(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.renaming.take() else {
            return;
        };
        let typed = draft.input.read(cx).text().trim().to_owned();
        let title = (!typed.is_empty()).then_some(typed);
        let pack_title = self.document_pack_title(&draft.document);
        // An open World is renamed through its session, so the name is
        // written by its own writer and never races a turn being saved; a
        // closed one in its file (refused if another app has it open).
        let renamed = match open_world_document(cx, &draft.document) {
            Some(document) => {
                let renamed = match document.try_borrow_mut() {
                    Ok(mut document) => {
                        let library = Arc::clone(&document.library);
                        document.session.rename(title.as_deref(), &library)
                    }
                    Err(_) => Err(LibraryError::InUse(self.library.path(&draft.document))),
                };
                refresh_world_windows(&document, cx);
                renamed
            }
            None => self
                .library
                .set_display_title(&draft.document, title.as_deref()),
        };
        match renamed {
            Ok(summary) => {
                mark_library_changed();
                let message = rename_result_message(summary.display_title.as_deref(), &pack_title);
                self.status = Some(match self.sync_documents_after_mutation() {
                    Some(error) => error,
                    None => HomeStatus::success(message),
                });
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not rename this World: {error}"
                )));
            }
        }
        cx.notify();
    }

    /// Removing a World is one click to ask and one to confirm; the card
    /// itself carries the question, so nothing is removed by a stray click.
    fn request_removal(&mut self, document_id: WorldDocumentId, cx: &mut Context<Self>) {
        self.renaming = None;
        self.pending_removal = Some(document_id);
        cx.notify();
    }

    fn cancel_removal(&mut self, cx: &mut Context<Self>) {
        self.pending_removal = None;
        cx.notify();
    }

    fn confirm_removal(&mut self, cx: &mut Context<Self>) {
        let Some(document_id) = self.pending_removal.take() else {
            return;
        };
        let title = self
            .document_title_for_id(&document_id)
            .unwrap_or_else(|| document_id.to_string());
        // An open World is not removed from under its window.
        if focus_open_world(cx, &document_id) {
            self.status = Some(HomeStatus::error(format!(
                "{title} is open: close its window first, then remove it."
            )));
            cx.notify();
            return;
        }
        let removed = self.library.remove(&document_id);
        match removed {
            Ok(path) => {
                mark_library_changed();
                diagnostics::info(format!("removed World {document_id} to {}", path.display()));
                self.status = Some(match self.sync_documents_after_mutation() {
                    Some(error) => error,
                    None => HomeStatus::success(removal_result_message(&title)),
                });
            }
            Err(error) => {
                self.status = Some(HomeStatus::error(format!(
                    "Could not remove this World: {error}"
                )));
            }
        }
        cx.notify();
    }

    fn document_title_for_id(&self, document_id: &WorldDocumentId) -> Option<String> {
        self.documents
            .iter()
            .find(|document| &document.id == document_id)
            .map(|document| {
                let pack_title = self
                    .registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone());
                self.distinct_title(document, &pack_title)
            })
    }

    /// A World's name on Home, told apart from any other of the same name:
    /// the second "Tiny Society" is "Tiny Society 2".
    fn distinct_title(&self, document: &WorldDocumentSummary, pack_title: &str) -> String {
        let title = world_summary_title(document, pack_title);
        let titled = |other: &WorldDocumentSummary| {
            let other_pack = self
                .registry
                .descriptor_for(&other.pack)
                .map(|descriptor| descriptor.title.clone())
                .unwrap_or_else(|| other.pack.id.clone());
            world_summary_title(other, &other_pack)
        };
        let mut same = self
            .documents
            .iter()
            .filter(|other| titled(other) == title)
            .map(|other| other.id.to_string())
            .collect::<Vec<_>>();
        same.sort();
        distinct(&title, &same, &document.id.to_string())
    }

    fn document_card(
        &self,
        document: WorldDocumentSummary,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open_id = document.id.clone();
        let export_id = document.id.clone();
        let rename_id = document.id.clone();
        let remove_id = document.id.clone();
        let pack_title = self
            .registry
            .descriptor_for(&document.pack)
            .map(|descriptor| descriptor.title.clone())
            .unwrap_or_else(|| document.pack.id.clone());
        let title = self.distinct_title(&document, &pack_title);
        let document_label = document.id.to_string();

        let mut details = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap_1()
            .child(ui::heading(title.clone()).truncate());
        // What a person needs to tell their Worlds apart: which kind of World
        // it is and how far it has lived. Event counts and file ids are the
        // archive's business; the World's own window still shows the file.
        details = details.child(ui::caption(world_card_meta(
            &title,
            &pack_title,
            document.world_time,
            document.display_calendar.as_ref(),
        )));

        let renaming_this_world = self
            .renaming
            .as_ref()
            .is_some_and(|draft| draft.document == document.id);
        let removing_this_world = self.pending_removal.as_ref() == Some(&document.id);
        if renaming_this_world {
            let input = self
                .renaming
                .as_ref()
                .expect("renaming_this_world implies a draft")
                .input
                .clone();
            details = details.child(
                div().w_full().flex().flex_col().gap_2().child(input).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            ui::button(
                                SharedString::from(format!("save-name-{document_label}")),
                                "Save name",
                                ui::ButtonKind::Secondary,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.commit_rename(cx))),
                        )
                        .child(
                            ui::button(
                                SharedString::from(format!("cancel-name-{document_label}")),
                                "Cancel",
                                ui::ButtonKind::Secondary,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_rename(cx))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .text_xs()
                                .text_color(ui::color(tokens::TEXT_SECONDARY))
                                .child(ui::t(
                                    "An empty name lists this World under its own title again.",
                                )),
                        ),
                ),
            );
        } else if removing_this_world {
            details = details.child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(ui::color(tokens::DANGER))
                            .child(format!(
                                "Remove {title}? Its file moves to the {} folder inside your Worlds folder, so you can put it back.",
                                world_library::REMOVED_DIRECTORY
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "confirm-remove-{document_label}"
                                    )))
                                    .cursor_pointer()
                                    .p_2()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(ui::color(tokens::DANGER))
                                    .bg(ui::color(tokens::DANGER_SOFT))
                                    .text_color(ui::color(tokens::DANGER))
                                    .text_sm()
                                    .child(ui::t("Remove"))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.confirm_removal(cx)),
                                    ),
                            )
                            .child(
                                div()
                                    .id(SharedString::from(format!(
                                        "keep-{document_label}"
                                    )))
                                    .cursor_pointer()
                                    .p_2()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(ui::color(tokens::BORDER_STRONG))
                                    .text_sm()
                                    .child(ui::t("Keep"))
                                    .on_click(cx.listener(|this, _, _, cx| this.cancel_removal(cx))),
                            ),
                    ),
            );
        }
        // What else can be done with a World waits behind its ⋯ and a
        // right-click, so the shelf is covers, not rows of links.
        let menu_open = self.card_menu.as_ref() == Some(&document.id)
            && !renaming_this_world
            && !removing_this_world;
        let menu =
            menu_open.then(|| {
                let item = |id: String, label: &'static str| {
                    div()
                        .id(SharedString::from(id))
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .text_sm()
                        .cursor_pointer()
                        .hover(|style| style.bg(ui::color(tokens::ROW_HOVER)))
                        .child(label)
                };
                div()
                    .id(SharedString::from(format!("menu-{document_label}")))
                    .absolute()
                    .top(px(6.0))
                    .right(px(44.0))
                    .w(px(200.0))
                    .p_1()
                    .rounded_lg()
                    .bg(ui::color(tokens::SURFACE))
                    .border_1()
                    .border_color(ui::color(tokens::BORDER))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(item(format!("rename-{document_label}"), "Rename").on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.card_menu = None;
                            this.begin_rename(rename_id.clone(), cx)
                        }),
                    ))
                    .child(
                        item(format!("export-{export_id}"), "Export…").on_click(cx.listener(
                            move |this, _, _, cx| {
                                this.card_menu = None;
                                this.export_document(export_id.clone(), cx)
                            },
                        )),
                    )
                    .child(
                        item(format!("remove-{document_label}"), "Remove")
                            .text_color(ui::color(tokens::DANGER))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.card_menu = None;
                                this.request_removal(remove_id.clone(), cx)
                            })),
                    )
            });

        // How long the World has been living without you, when it lives on
        // its own: what opening it will catch up on.
        let waiting = document
            .display_moves_alone
            .then(|| observer::periods_waiting(&document.id, &self.library))
            .flatten()
            .filter(|periods| *periods > 0);
        let unit = document
            .display_calendar
            .as_ref()
            .map(|calendar| calendar.unit.to_lowercase())
            .unwrap_or_else(|| "day".into());
        let mut cover = div()
            .id(SharedString::from(format!("cover-{document_label}")))
            .role(gpui::Role::Button)
            .aria_label(ui::t(format!("Open {title}")))
            .relative()
            .w_full()
            .h(px(WORLD_COVER_HEIGHT))
            .cursor_pointer()
            .child(if document.display_cast.is_empty() {
                ui::living_cover(
                    document.display_scenery.as_ref(),
                    &title,
                    document.id.as_str(),
                    &document.display_marks,
                )
                .size_full()
                .into_any_element()
            } else {
                // Its own people and buildings, as it last stood.
                world_gpui::diorama::cover(
                    document.display_scenery,
                    &document.display_marks,
                    document.display_cast.clone(),
                    document.display_drawings.clone(),
                )
                .size_full()
                .into_any_element()
            })
            .on_click(cx.listener({
                let open_id = open_id.clone();
                move |this, _, _, cx| this.open_document(open_id.clone(), cx)
            }));
        // The way in sits on the picture, the way a game shelf has it. The
        // click stops at the button, or the picture under it would open
        // the World a second time.
        cover = cover.child(
            div().absolute().bottom_3().right_3().child(
                ui::button(
                    SharedString::from(format!("open-{open_id}")),
                    "Open",
                    ui::ButtonKind::Primary,
                )
                .on_click(cx.listener({
                    let open_id = open_id.clone();
                    move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.open_document(open_id.clone(), cx)
                    }
                })),
            ),
        );
        if let Some(periods) = waiting {
            cover = cover.child(
                div()
                    .absolute()
                    .top_2()
                    .left_2()
                    .px_2()
                    .py_1()
                    .rounded_full()
                    .bg(ui::color(tokens::SURFACE))
                    .shadow_sm()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .size(px(6.0))
                            .rounded_full()
                            .bg(ui::color(tokens::SUCCESS)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT))
                            .child(time_waiting_line(periods, &unit)),
                    ),
            );
        }

        let menu_id = document.id.clone();
        cover = cover.child(
            div()
                .id(SharedString::from(format!("more-{document_label}")))
                .role(gpui::Role::Button)
                .aria_label(ui::t(format!("More for {title}")))
                .tooltip(ui::tip(format!("More for {title}")))
                .tooltip_show_delay(ui::TIP_DELAY)
                .aria_expanded(menu_open)
                .absolute()
                .top_2()
                .right_2()
                .size(px(30.0))
                .rounded_full()
                .bg(ui::color(tokens::SURFACE))
                .shadow_sm()
                .flex()
                .items_center()
                .justify_center()
                .text_color(ui::color(tokens::TEXT_SECONDARY))
                .cursor_pointer()
                .when(!menu_open, |more| {
                    more.opacity(0.0).group_hover(
                        SharedString::from(format!("card-{document_label}")),
                        |style| style.opacity(1.0),
                    )
                })
                .child("⋯")
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.card_menu = if this.card_menu.as_ref() == Some(&menu_id) {
                        None
                    } else {
                        Some(menu_id.clone())
                    };
                    cx.notify();
                })),
        );
        if let Some(menu) = menu {
            cover = cover.child(menu);
        }
        let right_click_id = document.id.clone();
        div()
            .id(SharedString::from(format!("document-{document_label}")))
            .role(gpui::Role::Group)
            .aria_label(title.clone())
            .group(SharedString::from(format!("card-{document_label}")))
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, _, _, cx| {
                    this.card_menu = Some(right_click_id.clone());
                    cx.notify();
                }),
            )
            .w_full()
            .min_w(px(0.0))
            .rounded_xl()
            .overflow_hidden()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .hover(|style| style.border_color(ui::color(tokens::BORDER_STRONG)))
            .flex()
            .flex_col()
            .child(cover)
            .child(div().p_4().child(details))
    }

    fn included_pack_is_installed(&self, pack: &WorldPackRef) -> bool {
        self.pack_catalog.as_ref().is_some_and(|catalog| {
            catalog
                .entries()
                .iter()
                .any(|installed| &installed.pack == pack)
        })
    }

    fn featured_included_pack_card(
        &self,
        pack: included_packs::IncludedPack,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let review_pack = pack.clone();
        let identity = format!("{} @ {}", pack.pack.id, pack.pack.version);
        div()
            .id("featured-included-world")
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::ACCENT_TEXT))
                            .child("START HERE · A WORLD THAT KEEPS LIVING"),
                    )
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(ui::color(tokens::ACCENT_TEXT))
                            .child(pack.experience),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::ACCENT_TEXT))
                            .child(format!(
                                "{identity} · Included external World · reviewed before it runs"
                            )),
                    ),
            )
            .child(
                ui::button(
                    "review-featured-included-world",
                    "Review & Start",
                    ui::ButtonKind::Secondary,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.review_included_pack(review_pack.clone(), true, cx)
                })),
            )
    }

    fn included_pack_card(
        &self,
        pack: included_packs::IncludedPack,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let review_pack = pack.clone();
        let identity = format!("{} @ {}", pack.pack.id, pack.pack.version);
        div()
            .id(SharedString::from(format!(
                "included-pack-{}-{}",
                pack.pack.id, pack.pack.version
            )))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::SUCCESS_SOFT))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(pack.experience),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child(format!(
                                "{identity} · Included external Pack · review required"
                            )),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from(format!(
                        "review-included-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER_STRONG))
                    .text_sm()
                    .child(ui::t("Review & Install"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.review_included_pack(review_pack.clone(), false, cx)
                    })),
            )
    }

    fn ready_pack_card(
        &self,
        descriptor: world_host::WorldDescriptor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pack_id = descriptor.pack.id.clone();
        let title = descriptor.title.clone();
        let button_title = format!("Create {}", descriptor.title);
        div()
            .id("pack-ready-to-create")
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .rounded_lg()
            .border_color(ui::color(tokens::ACCENT))
            .bg(ui::color(tokens::ACCENT_SOFT))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                // Without a basis of its own this column asks for the whole
                // sentence on one line and pushes the buttons past the window
                // edge at the size Home opens at, which put the app's own
                // first-run call to action half off screen. flex_1 alone is not
                // enough: a flex item's automatic minimum size is its content,
                // so the column still refuses to go narrower than the sentence
                // until min_w_0 says it may.
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(ui::heading(format!("{title} is ready")))
                    .child(ui::body(
                        "Start your first World. It keeps living between visits, and you can always create another.",
                    )),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .gap_2()
                    .child(
                        ui::button("create-ready-pack-world", button_title, ui::ButtonKind::Primary)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.create_world(pack_id.clone(), cx)
                            })),
                    )
                    .child(
                        ui::button("dismiss-ready-pack-world", "Not now", ui::ButtonKind::Secondary)
                            .on_click(cx.listener(|this, _, _, cx| this.dismiss_ready_pack(cx))),
                    ),
            )
    }

    fn pack_install_review_card(
        &self,
        preview: PackInstallPreview,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let start_after_install =
            start_after_install_matches(self.pending_start_after_install.as_ref(), preview.pack());
        let review_title = if start_after_install {
            "Before this World starts"
        } else {
            "A new Pack wants to install"
        };
        let confirm_title = if start_after_install {
            "Trust & Start"
        } else {
            "Install & Trust"
        };
        let format = preview.kind().label();
        let size = format_program_size(preview.program_bytes());
        let source = preview.source_path();
        let file = source
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| source.display().to_string());
        let version = preview.pack().version.clone();
        let identity = preview.pack().id.clone();
        let runtime = preview.runtime_name().to_owned();
        let sha = preview.program_sha256().to_owned();
        // The first and last few characters are what anyone compares.
        let fingerprint = if sha.len() > 16 {
            format!("{}…{}", &sha[..8], &sha[sha.len() - 8..])
        } else {
            sha.clone()
        };
        let fact = |label: &'static str, value: String| {
            div()
                .flex()
                .gap_3()
                .child(div().w(px(96.0)).flex_shrink_0().child(ui::caption(label)))
                .child(
                    div()
                        .min_w(px(0.0))
                        .flex_1()
                        .child(ui::body(value).truncate()),
                )
        };

        div()
            .id("pack-install-review")
            .w_full()
            .p_5()
            .rounded_lg()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .shadow_sm()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .gap_4()
                    .items_center()
                    .child(
                        div()
                            .size(px(64.0))
                            .flex_shrink_0()
                            .rounded_lg()
                            .overflow_hidden()
                            .child(ui::cover(preview.title(), &identity).size_full()),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(ui::section_label(review_title))
                            .child(ui::heading(preview.title().to_owned()))
                            .child(ui::caption(preview.description().to_owned())),
                    ),
            )
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(ui::color(tokens::WINDOW))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(fact("Version", version))
                    .child(fact("Runs as", format!("{runtime} · {format} · {size}")))
                    .child(fact("Fingerprint", fingerprint))
                    .child(fact("From", file))
                    .child(fact("Identity", identity)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size(px(8.0))
                            .flex_shrink_0()
                            .rounded_full()
                            .bg(ui::color(tokens::WARNING)),
                    )
                    .child(ui::body(if start_after_install {
                        "Nothing from this Pack has run yet. Trusting it approves exactly these bytes, then your World opens once its self-test passes."
                    } else {
                        "Nothing from this Pack has run yet. Trusting it approves exactly these bytes; any other version is refused."
                    })),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        ui::button("cancel-pack-install", "Cancel", ui::ButtonKind::Secondary)
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_pack_install(cx))),
                    )
                    .child(
                        ui::button("confirm-pack-install", confirm_title, ui::ButtonKind::Primary)
                            .on_click(cx.listener(|this, _, _, cx| this.confirm_pack_install(cx))),
                    ),
            )
    }

    fn installed_pack_card(&self, pack: InstalledPack, cx: &mut Context<Self>) -> impl IntoElement {
        let availability = self
            .pack_catalog
            .as_ref()
            .map(|catalog| catalog.availability(&pack.pack))
            .unwrap_or(PackAvailability::NotInstalled);
        let probing = self.is_pack_probing(&pack.pack);
        let state = if probing {
            "Testing durable round-trip…".to_string()
        } else {
            match &availability {
                PackAvailability::Ready if pack.active => "Active".to_string(),
                PackAvailability::Ready => "Historical · available for saved Worlds".to_string(),
                PackAvailability::Disabled => {
                    "Disabled · trusted, not runnable until tested".to_string()
                }
                PackAvailability::Invalid { reason } => format!("Invalid · {reason}"),
                PackAvailability::MissingVersion { .. } => "Missing exact version".to_string(),
                PackAvailability::NotInstalled => "Not installed".to_string(),
            }
        };
        let activate_pack = pack.pack.clone();
        let toggle_pack = pack.pack.clone();
        let test_pack = pack.pack.clone();
        let enabled = pack.enabled;
        let active = pack.active;

        let mut actions = div().flex().gap_2();
        if !probing && enabled && !active {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "activate-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER_STRONG))
                    .text_sm()
                    .child(ui::t("Activate"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.activate_pack(activate_pack.clone(), cx)
                    })),
            );
        }
        if !probing && enabled {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "toggle-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER_STRONG))
                    .text_sm()
                    .child(ui::t("Disable"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_pack_enabled(toggle_pack.clone(), false, cx)
                    })),
            );
        } else if !probing && !enabled {
            actions = actions.child(
                div()
                    .id(SharedString::from(format!(
                        "test-enable-pack-{}-{}",
                        pack.pack.id, pack.pack.version
                    )))
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER_STRONG))
                    .text_sm()
                    .child(ui::t("Test & Enable"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.start_pack_probe(test_pack.clone(), false, false, false, cx)
                    })),
            );
        }

        div()
            .id(SharedString::from(format!(
                "installed-pack-{}-{}",
                pack.pack.id, pack.pack.version
            )))
            .w_full()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(ui::color(tokens::BORDER_STRONG))
            .bg(ui::color(tokens::SURFACE))
            .flex()
            .justify_between()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_lg().child(pack.title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(ui::color(tokens::TEXT_SECONDARY))
                            .child(pack.description),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child(format!(
                                "{} @ {} · {state}",
                                pack.pack.id, pack.pack.version
                            )),
                    ),
            )
            .child(actions)
    }

    fn new_world_card(
        &self,
        descriptor: world_host::WorldDescriptor,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pack_id = descriptor.pack.id.clone();
        div()
            .id(SharedString::from(format!("new-world-{pack_id}")))
            .min_w(px(260.0))
            .flex_1()
            .p_4()
            .rounded_lg()
            .border_1()
            .border_color(ui::color(tokens::BORDER))
            .bg(ui::color(tokens::SURFACE))
            .cursor_pointer()
            .hover(|style| {
                style
                    .border_color(ui::color(tokens::ACCENT))
                    .bg(ui::color(tokens::SURFACE_HOVER))
            })
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .mb_2()
                    .h(px(84.0))
                    .w_full()
                    .rounded_md()
                    .overflow_hidden()
                    .child(ui::cover(&descriptor.title, &pack_id).size_full()),
            )
            .child(ui::row_title(descriptor.title))
            .child(ui::detail(descriptor.description).line_clamp(3))
            .child(
                div()
                    .pt_2()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(ui::color(tokens::ACCENT_TEXT))
                    .child(ui::t("Start a World →")),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.create_world(pack_id.clone(), cx)))
    }
}

#[cfg(gui)]
impl Render for WorldMachineHome {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        remember_window_geometry(window, RememberedWindow::Home);
        // The demo says it is one.
        window.set_window_title(demo::HOME_TITLE);

        // A World where nothing has happened yet is not one of My Worlds;
        // starting that kind of World again picks it back up.
        let documents = self
            .documents
            .iter()
            .filter(|document| has_begun(document))
            .cloned()
            .collect::<Vec<_>>();
        let has_documents = !documents.is_empty();
        let pack_filters = world_pack_filter_counts(&documents);
        let selected_world_pack = self.selected_world_pack.clone();
        let search_query = normalize_world_search(self.world_search.read(cx).text());
        let show_world_controls = documents.len() >= WORLD_SEARCH_THRESHOLD;
        let mut visible_cards = documents
            .iter()
            .filter(|document| world_matches_pack_filter(document, selected_world_pack.as_deref()))
            .map(|document| {
                let pack_title = self
                    .registry
                    .descriptor_for(&document.pack)
                    .map(|descriptor| descriptor.title.clone())
                    .unwrap_or_else(|| document.pack.id.clone());
                let title = world_summary_title(document, &pack_title);
                (title, pack_title, document.clone())
            })
            .filter(|(title, pack_title, document)| {
                world_matches_search(title, pack_title, document.id.as_str(), &search_query)
            })
            .map(|(title, _pack_title, document)| (title, document))
            .collect::<Vec<_>>();
        sort_world_cards(&mut visible_cards, self.world_sort);
        let visible_documents = visible_cards
            .into_iter()
            .map(|(_title, document)| document)
            .collect::<Vec<_>>();
        let visible_document_count = visible_documents.len();
        let descriptors = self
            .registry
            .descriptors()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let first_run = !has_documents;
        let featured_included = self
            .included_packs
            .iter()
            .find(|pack| pack.featured && !self.included_pack_is_installed(&pack.pack))
            .filter(|pack| !demo::ENABLED || demo::offers_pack(&pack.pack.id))
            .cloned();
        let featured_review_pending = self
            .pending_pack_install
            .as_ref()
            .zip(featured_included.as_ref())
            .is_some_and(|(preview, featured)| preview.pack() == &featured.pack);
        let show_featured = first_run
            && self.pending_pack_install.is_none()
            && self.ready_pack_to_create.is_none()
            && featured_included.is_some();

        let mut saved = div().w_full().flex().flex_col().gap_3();
        if !has_documents {
            saved = saved.child(
                div()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER))
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(ui::t(
                        "No Worlds yet. Start one below; it keeps living while you are away.",
                    )),
            );
        } else if visible_documents.is_empty() {
            let empty = if search_query.is_empty() {
                "No Worlds match this Pack filter."
            } else {
                "No Worlds match what you typed."
            };
            saved = saved.child(
                div()
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(ui::color(tokens::BORDER))
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(empty),
            );
        } else {
            // Two columns that each stack their own cards, so a tall card
            // never leaves a hole beside a short one.
            let mut columns = [
                div().flex_1().min_w(px(0.0)).flex().flex_col().gap_4(),
                div().flex_1().min_w(px(0.0)).flex().flex_col().gap_4(),
            ];
            for (index, document) in visible_documents.into_iter().enumerate() {
                let column = std::mem::replace(&mut columns[index % 2], div());
                columns[index % 2] = column.child(self.document_card(document, cx));
            }
            let [left, right] = columns;
            saved = saved.child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap_4()
                    .child(left)
                    .child(right),
            );
        }

        let mut world_filters = div().w_full().flex().gap_2();
        if pack_filters.len() > 1 {
            let all_selected = selected_world_pack.is_none();
            let (all_border, all_background, all_text) = if all_selected {
                (0x6f86b0, 0xe9eef7, 0x314b72)
            } else {
                (0xd9d9d3, 0xffffff, 0x666666)
            };
            world_filters = world_filters.child(
                div()
                    .id("world-pack-filter-all")
                    .cursor_pointer()
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(crate::theme_rgb(all_border))
                    .bg(crate::theme_rgb(all_background))
                    .text_color(crate::theme_rgb(all_text))
                    .text_xs()
                    .child(format!("All · {}", documents.len()))
                    .on_click(cx.listener(|this, _, _, cx| this.set_world_pack_filter(None, cx))),
            );
            for (pack_id, count) in pack_filters.iter() {
                let selected = selected_world_pack.as_deref() == Some(pack_id.as_str());
                let (border, background, text) = if selected {
                    (0x6f86b0, 0xe9eef7, 0x314b72)
                } else {
                    (0xd9d9d3, 0xffffff, 0x666666)
                };
                let filter_pack = pack_id.clone();
                let filter_title = self.pack_filter_title(pack_id);
                world_filters = world_filters.child(
                    div()
                        .id(SharedString::from(format!("world-pack-filter-{pack_id}")))
                        .cursor_pointer()
                        .p_2()
                        .rounded_md()
                        .border_1()
                        .border_color(crate::theme_rgb(border))
                        .bg(crate::theme_rgb(background))
                        .text_color(crate::theme_rgb(text))
                        .text_xs()
                        .child(format!("{filter_title} · {count}"))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_world_pack_filter(Some(filter_pack.clone()), cx)
                        })),
                );
            }
        }

        let visible_included_packs = self
            .included_packs
            .iter()
            .filter(|included| !self.included_pack_is_installed(&included.pack))
            .filter(|included| !demo::ENABLED || demo::offers_pack(&included.pack.id))
            .filter(|included| {
                featured_included.as_ref().is_none_or(|featured| {
                    featured.pack != included.pack || (!show_featured && !featured_review_pending)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut included = div().w_full().flex().flex_col().gap_3();
        for pack in visible_included_packs.iter().cloned() {
            included = included.child(self.included_pack_card(pack, cx));
        }

        // The demo has no Packs to show beyond its own.
        let installed_packs = self
            .pack_catalog
            .as_ref()
            .filter(|_| !demo::ENABLED)
            .map(|catalog| catalog.entries().to_vec())
            .unwrap_or_default();
        let mut installed = div().w_full().flex().flex_col().gap_3();
        for pack in installed_packs.iter().cloned() {
            installed = installed.child(self.installed_pack_card(pack, cx));
        }

        let mut available = div().w_full().flex().flex_wrap().gap_3();
        // The World a newcomer should try first leads the list.
        let mut descriptors = descriptors;
        descriptors.sort_by_key(|descriptor| {
            !self
                .included_packs
                .iter()
                .any(|included| included.featured && included.pack.id == descriptor.pack.id)
        });
        // The demo offers its one Pack, and no other.
        for descriptor in descriptors
            .into_iter()
            .filter(|descriptor| !demo::ENABLED || demo::offers_pack(&descriptor.pack.id))
        {
            available = available.child(self.new_world_card(descriptor, cx));
        }

        let header = div()
            .id("world-machine-home-chrome")
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(ui::page_title(demo::HOME_TITLE))
            .child(ui::body(
                "Small worlds that keep living while you are away.",
            ));

        let mut body = div()
            .w_full()
            .max_w(px(760.0))
            .mx_auto()
            .flex()
            .flex_col()
            .gap_4()
            .px_6()
            .pt_8()
            .pb_10()
            .child(header);

        if let Some(update) = self.available_update.clone() {
            body = body.child(self.update_banner(update, cx));
        }

        if let Some(preview) = self.pending_pack_install.clone() {
            body = body.child(self.pack_install_review_card(preview, cx));
        }

        if let Some(descriptor) = self.ready_pack_descriptor() {
            body = body.child(self.ready_pack_card(descriptor, cx));
        }

        if show_featured {
            let featured = featured_included.expect("show_featured requires a featured Pack");
            body = body
                .child(home_section_title("Start here"))
                .child(self.featured_included_pack_card(featured, cx));
        }

        if has_documents || self.included_packs.is_empty() {
            body = body.child(home_section_title(my_worlds_title(
                visible_document_count,
                documents.len(),
            )));
            if show_world_controls {
                body = body.child(self.world_controls(cx));
            }
            if let Some(note) = unreadable_documents_note(&self.unreadable_documents) {
                body = body.child(
                    div()
                        .w_full()
                        .p_3()
                        .rounded_md()
                        .border_1()
                        .border_color(ui::color(tokens::DANGER))
                        .bg(ui::color(tokens::DANGER_SOFT))
                        .text_xs()
                        .text_color(ui::color(tokens::DANGER))
                        .child(note),
                );
            }
            if show_world_controls && pack_filters.len() > 1 {
                body = body.child(world_filters);
            }
            body = body.child(saved);
        }

        if !visible_included_packs.is_empty() {
            body = body
                .child(home_section_title(if first_run {
                    "More worlds"
                } else {
                    "Included Worlds"
                }))
                .child(included);
        }

        body = body
            .child(home_section_title("Start a new World"))
            .child(available);

        if !installed_packs.is_empty() {
            let packs_title = if self.show_packs {
                format!("Packs · {} · hide", installed_packs.len())
            } else {
                format!("Packs · {} · show", installed_packs.len())
            };
            body = body.child(
                div()
                    .id("toggle-installed-packs")
                    .cursor_pointer()
                    .text_sm()
                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                    .child(packs_title)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_packs = !this.show_packs;
                        cx.notify();
                    })),
            );
            if self.show_packs {
                body = body.child(installed);
            }
        }

        let mut shell = div()
            .relative()
            .size_full()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .flex()
            .flex_col()
            .child(
                div()
                    .id("world-machine-home-scroll")
                    .w_full()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(body),
            );

        // Status floats over the bottom of the window instead of being
        // inserted above the page, so a Pack finishing its start-up never
        // moves the card under the pointer.
        if let Some(status) = &self.status {
            let tone = match status.tone {
                HomeStatusTone::Info => tokens::ACCENT_TEXT,
                HomeStatusTone::Success => tokens::SUCCESS,
                HomeStatusTone::Error => tokens::DANGER,
            };
            shell = shell.child(
                div()
                    .id("world-machine-home-status")
                    .absolute()
                    .bottom_4()
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .px_6()
                    .child(
                        div()
                            .max_w(px(640.0))
                            .w_full()
                            .pl_4()
                            .pr_2()
                            .py_2()
                            .rounded_lg()
                            .border_1()
                            .border_color(ui::color(tokens::BORDER_STRONG))
                            .bg(ui::color(tokens::SURFACE))
                            .shadow_md()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .text_sm()
                                    .text_color(ui::color(tone))
                                    .child(status.message.clone()),
                            )
                            .child(
                                ui::button(
                                    "dismiss-world-machine-home-status",
                                    "Dismiss",
                                    ui::ButtonKind::Secondary,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.status = None;
                                        cx.notify();
                                    },
                                )),
                            ),
                    ),
            );
        }

        shell
    }
}

/// How My Worlds is ordered.
#[cfg(gui)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorldSort {
    /// The Library's own order: most recently played first.
    Recent,
    /// What the cards say, A to Z.
    Name,
}

#[cfg(gui)]
impl WorldSort {
    fn label(self) -> &'static str {
        match self {
            Self::Recent => "Recent",
            Self::Name => "Name",
        }
    }
}

/// Beyond this many Worlds the list needs finding and ordering; below it the
/// controls would be clutter on a screen that shows every World at once.
#[cfg(gui)]
const WORLD_SEARCH_THRESHOLD: usize = 10;

#[cfg(gui)]
fn normalize_world_search(query: &str) -> String {
    query.trim().to_lowercase()
}

/// A World matches what was typed when the text appears in something the card
/// itself shows: its name, its World Pack's title, or its file identity.
#[cfg(gui)]
fn world_matches_search(title: &str, pack_title: &str, document_id: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    [title, pack_title, document_id]
        .iter()
        .any(|field| field.to_lowercase().contains(query))
}

/// Order the cards. `Recent` leaves the Library's own most-recently-played
/// order alone; `Name` sorts by what the card shows, ignoring case, with the
/// file identity breaking ties so the order never wobbles between renders.
#[cfg(gui)]
fn sort_world_cards(cards: &mut [(String, WorldDocumentSummary)], sort: WorldSort) {
    if sort == WorldSort::Name {
        cards.sort_by(|(left_title, left), (right_title, right)| {
            left_title
                .to_lowercase()
                .cmp(&right_title.to_lowercase())
                .then_with(|| left.id.cmp(&right.id))
        });
    }
}

/// What the My Worlds heading says once some Worlds are filtered out.
#[cfg(gui)]
fn my_worlds_title(visible: usize, total: usize) -> String {
    if visible == total {
        format!("My Worlds · {total}")
    } else {
        format!("My Worlds · {visible}/{total}")
    }
}

/// The placeholder in the name field: an unnamed World shows the World Pack's
/// own title, which is exactly what the card falls back to.
#[cfg(gui)]
fn rename_placeholder(pack_title: &str) -> String {
    format!("{pack_title} — name this World")
}

#[cfg(gui)]
fn rename_result_message(display_title: Option<&str>, pack_title: &str) -> String {
    // A World open in a window is renamed through its own session (its
    // writer writes the name in turn with its turns), so nothing there needs
    // reloading.
    match display_title {
        Some(title) => format!("Renamed to {title}."),
        None => format!("Name cleared; this World is listed as {pack_title} again."),
    }
}

#[cfg(gui)]
fn removal_result_message(title: &str) -> String {
    format!(
        "Removed {title}. Its file moved to the {} folder inside your Worlds folder.",
        world_library::REMOVED_DIRECTORY
    )
}

#[cfg(gui)]
const UNREADABLE_DOCUMENT_NAME_LIMIT: usize = 3;

/// Name the files that could not be read, without letting a folder full of
/// them push every World off the screen.
#[cfg(gui)]
fn unreadable_documents_note(unreadable: &[UnreadableWorldFile]) -> Option<String> {
    if unreadable.is_empty() {
        return None;
    }
    let named = unreadable
        .iter()
        .take(UNREADABLE_DOCUMENT_NAME_LIMIT)
        .map(|file| file.file_name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let hidden = unreadable
        .len()
        .saturating_sub(UNREADABLE_DOCUMENT_NAME_LIMIT);
    let names = if hidden > 0 {
        format!("{named}, and {hidden} more")
    } else {
        named
    };
    Some(if unreadable.len() == 1 {
        format!("One file in your Worlds folder could not be read and is not listed: {names}.")
    } else {
        format!(
            "{} files in your Worlds folder could not be read and are not listed: {names}.",
            unreadable.len()
        )
    })
}

#[cfg(gui)]
fn report_unreadable_documents(unreadable: &[UnreadableWorldFile]) {
    for file in unreadable {
        diagnostics::error(format!(
            "could not read {} in the Worlds folder: {}",
            file.file_name, file.reason
        ));
    }
}

/// A heading over one group of cards on Home.
#[cfg(gui)]
fn home_section_title(text: impl Into<SharedString>) -> gpui::Div {
    div().pt_4().child(ui::heading(text))
}

/// Whether anything has happened in a World yet.
#[cfg(gui)]
fn has_begun(document: &WorldDocumentSummary) -> bool {
    document.event_count > 0 || document.world_time > 0
}

/// How tall a World's cover stands on Home.
#[cfg(gui)]
const WORLD_COVER_HEIGHT: f32 = 196.0;

/// How long a World has been living without you, in its own unit:
/// "1 sol has passed", "3 nights have passed".
#[cfg(gui)]
fn time_waiting_line(periods: u64, unit: &str) -> String {
    world_gpui::i18n::time_passed(periods, unit)
}

/// "1 sol", "3 nights".
#[cfg(gui)]
fn count_of(count: u64, unit: &str) -> String {
    world_gpui::i18n::count_of(count, unit)
}

/// The line under a World's name on Home.
#[cfg(gui)]
fn world_card_meta(
    title: &str,
    pack_title: &str,
    world_time: u64,
    calendar: Option<&world_projection::Calendar>,
) -> String {
    // The World's own count ("Sol 5"), the way its window says it.
    let age = if world_time == 0 {
        ui::t("just begun").to_string()
    } else {
        world_gpui::i18n::moment_label(
            &world_projection::ProjectionSnapshot {
                calendar: calendar.cloned(),
                ..world_projection::ProjectionSnapshot::default()
            },
            world_time,
        )
    };
    if title == pack_title {
        age
    } else {
        format!("{} · {age}", ui::t(pack_title.to_string()))
    }
}

#[cfg(gui)]
fn world_summary_title(document: &WorldDocumentSummary, pack_title: &str) -> String {
    document
        .display_title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(pack_title)
        .to_owned()
}

/// `title` for the World `id`, of the Worlds `same` (by id, oldest
/// first) that share it: the first keeps it, the next are numbered.
#[cfg(gui)]
fn distinct(title: &str, same: &[String], id: &str) -> String {
    match same.iter().position(|other| other == id) {
        Some(at) if at > 0 => format!("{title} {}", at + 1),
        _ => title.to_owned(),
    }
}

#[cfg(gui)]
fn world_matches_pack_filter(document: &WorldDocumentSummary, pack_id: Option<&str>) -> bool {
    pack_id.is_none_or(|pack_id| document.pack.id == pack_id)
}

#[cfg(gui)]
fn world_pack_filter_is_available(
    documents: &[WorldDocumentSummary],
    pack_id: Option<&str>,
) -> bool {
    pack_id.is_none_or(|pack_id| documents.iter().any(|document| document.pack.id == pack_id))
}

#[cfg(gui)]
fn world_pack_filter_counts(documents: &[WorldDocumentSummary]) -> Vec<(String, usize)> {
    let mut filters = Vec::<(String, usize)>::new();
    for document in documents {
        if let Some((_pack_id, count)) = filters
            .iter_mut()
            .find(|(pack_id, _count)| pack_id == &document.pack.id)
        {
            *count += 1;
        } else {
            filters.push((document.pack.id.clone(), 1));
        }
    }
    filters
}

/// What a World is called: the name its owner gave it, or the durable identity
/// of its file when it has no name.
#[cfg(gui)]
fn document_display_name(display_title: Option<&str>, durable_label: &str) -> String {
    display_title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(durable_label)
        .to_owned()
}

#[cfg(gui)]
fn session_display_name(session: &DurableWorldSession) -> String {
    let durable_label = session.display_name();
    document_display_name(session.metadata().display_title.as_deref(), &durable_label)
}

#[cfg(gui)]
fn document_window_title(document_label: &str) -> String {
    format!("{document_label} — World Machine")
}

#[cfg(gui)]
fn start_after_install_matches(pending: Option<&WorldPackRef>, pack: &WorldPackRef) -> bool {
    pending == Some(pack)
}

#[cfg(gui)]
fn format_program_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    if bytes >= MIB {
        format!("{:.1} MiB · {bytes} bytes", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB · {bytes} bytes", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} bytes")
    }
}

#[cfg(all(test, gui))]
mod file_type_tests {
    use super::*;

    #[test]
    fn finder_knows_every_file_the_app_opens_and_what_it_is() {
        let plist = include_str!("../macos/Info.plist.in");
        for suffix in [
            WORLD_DOCUMENT_SUFFIX,
            world_library::WORLD_CODE_SUFFIX,
            PACK_BUNDLE_SUFFIX,
        ] {
            let extension = suffix.trim_start_matches('.');
            assert!(
                plist.contains(&format!("<string>{extension}</string>")),
                "{extension} is declared"
            );
        }
        // A World file is gzip, not JSON; a World code is text.
        assert!(plist.contains("<string>org.gnu.gnu-zip-archive</string>"));
        assert!(!plist.contains("public.json"));
        assert!(plist.contains("<string>public.utf8-plain-text</string>"));
        // What Finder hands the app goes down the Open menu's own paths.
        assert!(sharing::is_world_code_file(Path::new("/tmp/a.worldcode")));
        assert!(is_world_file(Path::new("/tmp/a.world")));
        let (url, path) = if cfg!(windows) {
            (
                "file:///C:/tmp/Leo%27s%20town.worldcode",
                r"C:\tmp\Leo's town.worldcode",
            )
        } else {
            (
                "file:///tmp/Leo%27s%20town.worldcode",
                "/tmp/Leo's town.worldcode",
            )
        };
        assert_eq!(
            system_open::path_from_open_url(url).unwrap(),
            PathBuf::from(path)
        );
    }

    #[test]
    fn document_status_tone_is_explicit_not_inferred_from_message_text() {
        let success = DocumentStatus::success("failed-looking note after a durable save");
        let error = DocumentStatus::error("failed");

        assert_eq!(success.tone, DocumentStatusTone::Success);
        assert_eq!(error.tone, DocumentStatusTone::Error);
        assert_eq!(success.message, "failed-looking note after a durable save");
    }

    #[test]
    fn library_change_revision_advances_after_mark() {
        let before = library_change_revision();
        mark_library_changed();
        assert!(library_change_revision() > before);
    }

    #[test]
    fn home_status_tone_is_explicit_not_inferred_from_message_text() {
        let info = HomeStatus::info("Could not-looking informational text");
        let success = HomeStatus::success("done");
        let error = HomeStatus::error("failed");

        assert_eq!(info.tone, HomeStatusTone::Info);
        assert_eq!(success.tone, HomeStatusTone::Success);
        assert_eq!(error.tone, HomeStatusTone::Error);
        assert_eq!(info.message, "Could not-looking informational text");
    }

    #[test]
    fn world_pack_filters_preserve_recent_pack_order_and_counts() {
        let summary = |id: &str, pack_id: &str| WorldDocumentSummary {
            id: WorldDocumentId::new(id).unwrap(),
            pack: WorldPackRef::new(pack_id, "1.0.0"),
            display_title: None,
            display_summary: None,
            display_scenery: None,
            display_calendar: None,
            display_marks: Vec::new(),
            display_moves_alone: false,
            display_cast: Vec::new(),
            display_drawings: Vec::new(),
            world_time: 0,
            event_count: 0,
        };
        let documents = vec![
            summary("pocket-new", "pocket-universe"),
            summary("tiny", "tiny-society"),
            summary("pocket-old", "pocket-universe"),
            summary("company", "micro-company"),
        ];

        assert_eq!(
            world_pack_filter_counts(&documents),
            vec![
                ("pocket-universe".into(), 2),
                ("tiny-society".into(), 1),
                ("micro-company".into(), 1),
            ]
        );
        assert!(world_matches_pack_filter(&documents[0], None));
        assert!(world_matches_pack_filter(
            &documents[0],
            Some("pocket-universe")
        ));
        assert!(!world_matches_pack_filter(
            &documents[1],
            Some("pocket-universe")
        ));
        assert!(world_pack_filter_is_available(
            &documents,
            Some("tiny-society")
        ));
        assert!(!world_pack_filter_is_available(
            &documents,
            Some("missing-pack")
        ));
    }

    #[test]
    fn home_counts_a_world_in_its_own_time_and_says_what_waits() {
        let sols = world_projection::Calendar {
            unit: "Sol".into(),
            length: 10,
            season: None,
            coming: None,
            festival_today: false,
            year: None,
        };
        assert_eq!(
            world_card_meta("Ares Pocket Colony", "Pocket Universe", 50, Some(&sols)),
            "Pocket Universe · Sol 5"
        );
        assert_eq!(
            world_card_meta("Pocket Universe", "Pocket Universe", 0, Some(&sols)),
            "just begun"
        );
        assert_eq!(time_waiting_line(1, "sol"), "1 sol has passed");
        assert_eq!(time_waiting_line(3, "night"), "3 nights have passed");
        assert_eq!(count_of(2, "day"), "2 days");
    }

    #[test]
    fn a_world_nothing_has_happened_in_is_not_listed_yet() {
        let mut summary = WorldDocumentSummary {
            id: WorldDocumentId::new("fresh").unwrap(),
            pack: WorldPackRef::new("world-machine.pocket-universe", "0.25.0"),
            display_title: None,
            display_summary: None,
            display_scenery: None,
            display_calendar: None,
            display_marks: Vec::new(),
            display_moves_alone: false,
            display_cast: Vec::new(),
            display_drawings: Vec::new(),
            world_time: 0,
            event_count: 0,
        };
        assert!(!has_begun(&summary));
        summary.event_count = 3;
        assert!(has_begun(&summary));
    }

    #[test]
    fn a_world_says_when_it_next_moves_on_its_own() {
        assert_eq!(
            keeps_going_line(5 * 3600 + 10, "sol"),
            "Keeps going without you · next sol in 6 h"
        );
        assert_eq!(
            keeps_going_line(20 * 60, "night"),
            "Keeps going without you · next night in 20 min"
        );
        assert_eq!(
            keeps_going_line(0, "day"),
            "Keeps going without you · a new day waits for your next visit"
        );
    }

    #[test]
    fn worlds_of_one_name_are_told_apart() {
        let same = ["tiny-society-1".to_string(), "tiny-society-2".to_string()];
        assert_eq!(
            distinct("Tiny Society", &same, "tiny-society-1"),
            "Tiny Society"
        );
        assert_eq!(
            distinct("Tiny Society", &same, "tiny-society-2"),
            "Tiny Society 2"
        );
        assert_eq!(distinct("Ares", &["ares".to_string()], "ares"), "Ares");
    }

    #[test]
    fn world_summary_title_prefers_semantic_title_and_falls_back_cleanly() {
        let pack = WorldPackRef::new("pocket-universe", "0.10.0");
        let mut summary = WorldDocumentSummary {
            id: WorldDocumentId::new("mars").unwrap(),
            pack,
            display_title: Some("  Ares Pocket Colony  ".into()),
            display_summary: Some("  Current thread · Ridge Network  ".into()),
            display_scenery: None,
            display_calendar: None,
            display_marks: Vec::new(),
            display_moves_alone: false,
            display_cast: Vec::new(),
            display_drawings: Vec::new(),
            world_time: 3,
            event_count: 7,
        };

        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Ares Pocket Colony"
        );
        summary.display_title = Some("   ".into());
        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Pocket Universe"
        );
        summary.display_title = None;
        assert_eq!(
            world_summary_title(&summary, "Pocket Universe"),
            "Pocket Universe"
        );
    }

    #[test]
    fn finding_a_world_matches_what_the_card_shows() {
        let query = normalize_world_search("  MAPLE  ");
        assert_eq!(query, "maple");

        assert!(world_matches_search(
            "Maple Street · 1987",
            "Pocket Universe",
            "pocket-universe-3",
            &query
        ));
        assert!(world_matches_search(
            "Ares Colony",
            "Pocket Universe",
            "maple-import",
            &query
        ));
        assert!(!world_matches_search(
            "Ares Colony",
            "Pocket Universe",
            "pocket-universe-3",
            &query
        ));

        // An empty box hides nothing.
        assert!(world_matches_search("Ares", "Tiny Society", "tiny-1", ""));
        // The World Pack's own title is searchable too.
        assert!(world_matches_search(
            "Ares",
            "Tiny Society",
            "tiny-1",
            &normalize_world_search("tiny")
        ));
    }

    #[test]
    fn ordering_by_name_ignores_case_and_stays_stable() {
        let card = |id: &str, title: &str| {
            (
                title.to_owned(),
                WorldDocumentSummary {
                    id: WorldDocumentId::new(id).unwrap(),
                    pack: WorldPackRef::new("pocket-universe", "1.0.0"),
                    display_title: Some(title.to_owned()),
                    display_summary: None,
                    display_scenery: None,
                    display_calendar: None,
                    display_marks: Vec::new(),
                    display_moves_alone: false,
                    display_cast: Vec::new(),
                    display_drawings: Vec::new(),
                    world_time: 0,
                    event_count: 0,
                },
            )
        };
        let recent_order = vec![
            card("c", "zephyr"),
            card("a", "Maple Street"),
            card("b", "ares colony"),
            card("d", "Maple Street"),
        ];

        let mut untouched = recent_order.clone();
        sort_world_cards(&mut untouched, WorldSort::Recent);
        assert_eq!(
            untouched
                .iter()
                .map(|(_, document)| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["c", "a", "b", "d"],
            "Recent must leave the Library's own order alone"
        );

        let mut by_name = recent_order;
        sort_world_cards(&mut by_name, WorldSort::Name);
        assert_eq!(
            by_name
                .iter()
                .map(|(_, document)| document.id.as_str())
                .collect::<Vec<_>>(),
            vec!["b", "a", "d", "c"],
            "equal names fall back to the file identity so the order is stable"
        );
    }

    #[test]
    fn the_my_worlds_heading_counts_what_is_hidden() {
        assert_eq!(my_worlds_title(7, 7), "My Worlds · 7");
        assert_eq!(my_worlds_title(2, 7), "My Worlds · 2/7");
        assert_eq!(my_worlds_title(0, 7), "My Worlds · 0/7");
    }

    #[test]
    fn renaming_reports_the_name_or_the_pack_title_it_fell_back_to() {
        let named = rename_result_message(Some("Maple Street"), "Pocket Universe");
        assert_eq!(named, "Renamed to Maple Street.");
        assert!(
            !named.contains("Reload"),
            "an open World is renamed in place"
        );
        let cleared = rename_result_message(None, "Pocket Universe");
        assert_eq!(
            cleared,
            "Name cleared; this World is listed as Pocket Universe again."
        );
        assert_eq!(
            rename_placeholder("Pocket Universe"),
            "Pocket Universe — name this World"
        );
    }

    #[test]
    fn removal_says_where_the_file_went() {
        let message = removal_result_message("Maple Street");
        assert!(message.starts_with("Removed Maple Street."));
        assert!(message.contains(world_library::REMOVED_DIRECTORY));
    }

    #[test]
    fn unreadable_files_are_named_without_crowding_out_the_worlds() {
        let file = |name: &str| UnreadableWorldFile {
            file_name: name.into(),
            reason: "unexpected end of input".into(),
        };

        assert_eq!(unreadable_documents_note(&[]), None);
        assert_eq!(
            unreadable_documents_note(&[file("truncated.world")]).as_deref(),
            Some(
                "One file in your Worlds folder could not be read and is not listed: truncated.world."
            )
        );
        assert_eq!(
            unreadable_documents_note(&[file("a.world"), file("b.world")]).as_deref(),
            Some("2 files in your Worlds folder could not be read and are not listed: a.world, b.world.")
        );
        assert_eq!(
            unreadable_documents_note(&[
                file("a.world"),
                file("b.world"),
                file("c.world"),
                file("d.world"),
                file("e.world"),
            ])
            .as_deref(),
            Some(
                "5 files in your Worlds folder could not be read and are not listed: a.world, b.world, c.world, and 2 more."
            )
        );
    }

    #[test]
    fn suggested_world_file_name_prefers_semantic_unicode_title() {
        assert_eq!(
            suggested_world_file_name("  Maple Street · 1987  ", "pocket-universe-42"),
            "Maple Street · 1987.world"
        );
        assert_eq!(
            suggested_world_file_name("Ares / Ice: Colony?", "pocket-universe-42"),
            "Ares - Ice- Colony.world"
        );
    }

    #[test]
    fn suggested_world_file_name_falls_back_to_durable_identity() {
        assert_eq!(
            suggested_world_file_name("   ", "pocket-universe-42"),
            "pocket-universe-42.world"
        );
        assert_eq!(
            suggested_world_file_name("", "legacy.world.json"),
            "legacy.world"
        );
    }

    #[test]
    fn a_world_is_called_by_its_name_and_falls_back_to_its_durable_identity() {
        assert_eq!(
            document_display_name(Some("  Maple Street · 1987  "), "pocket-universe-42"),
            "Maple Street · 1987"
        );
        assert_eq!(
            document_display_name(Some("   "), "pocket-universe-42"),
            "pocket-universe-42"
        );
        assert_eq!(
            document_display_name(None, "pocket-universe-42"),
            "pocket-universe-42"
        );
    }

    #[test]
    fn document_window_title_carries_what_the_world_is_called() {
        assert_eq!(
            document_window_title("pocket-universe-42"),
            "pocket-universe-42 — World Machine"
        );
        assert_eq!(
            document_window_title("Maple Street · 1987"),
            "Maple Street · 1987 — World Machine"
        );
    }

    #[test]
    fn start_intent_is_bound_to_exact_pack_identity() {
        let pocket_010 = WorldPackRef::new("pocket-universe", "0.10.0");
        let same = WorldPackRef::new("pocket-universe", "0.10.0");
        let newer = WorldPackRef::new("pocket-universe", "0.11.0");
        let other = WorldPackRef::new("micro-company", "0.10.0");

        assert!(start_after_install_matches(Some(&pocket_010), &same));
        assert!(!start_after_install_matches(Some(&pocket_010), &newer));
        assert!(!start_after_install_matches(Some(&pocket_010), &other));
        assert!(!start_after_install_matches(None, &pocket_010));
    }

    #[test]
    fn system_open_distinguishes_world_documents_from_portable_packs() {
        assert!(is_world_file(Path::new("/tmp/example.world")));
        assert!(!is_world_pack_file(Path::new("/tmp/example.world")));

        assert!(is_world_pack_file(Path::new("/tmp/example.worldpack")));
        assert!(!is_world_file(Path::new("/tmp/example.worldpack")));

        assert!(!is_world_pack_file(Path::new(
            "/tmp/example.worldpack.backup"
        )));
        assert!(!is_world_pack_file(Path::new(
            "/tmp/example.world-pack.json"
        )));
    }
}

#[cfg(gui)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gpui_platform::application;

    let application = application();
    system_open::install(&application);
    diagnostics::init();
    diagnostics::info(world_machine_desktop::steam::start().describe());
    // Every World file this app writes says which app wrote it.
    world_document::set_writing_app(format!("World Machine {}", build_info::APP_VERSION));
    load_window_geometry();
    let saved = world_machine_desktop::app_settings::application_support_root()
        .ok()
        .and_then(|root| world_machine_desktop::app_settings::load(&root).ok());
    // The app's words and the built-in Worlds' in Simplified Chinese, and
    // the language, text size and contrast the player chose.
    world_i18n::install(world_gpui::i18n::APP_ZH_HANS);
    for catalog in world_builtins::ZH_HANS {
        world_i18n::install(catalog);
    }
    world_i18n::install(&world_builtins::zh_hans_voices());
    // The app's own words last: where a Pack translates the same English
    // for its scene ("Open" as a shop is, 营业中), the app's buttons keep
    // theirs ("Open" a World, 打开).
    world_i18n::install(world_gpui::i18n::APP_ZH_HANS);
    // And the same again in Japanese.
    use world_i18n::{install_for, Language::Japanese};
    for catalog in world_builtins::JA {
        install_for(Japanese, catalog);
    }
    install_for(Japanese, &world_builtins::ja_voices());
    install_for(Japanese, world_gpui::i18n::APP_JA);
    world_machine_desktop::display::apply(saved.as_ref());
    install_pointers(saved.as_ref());
    ambience::set_enabled(
        saved
            .as_ref()
            .is_some_and(|settings| settings.ambient_sound),
    );
    if let Some(settings) = &saved {
        for channel in ambience::Channel::ALL {
            ambience::set_level(channel, settings.sound_level(channel));
        }
    }
    let library = Arc::new(discover_library()?);
    let pack_catalog_path = discover_pack_catalog_path(library.as_ref());
    diagnostics::info(format!(
        "worlds at {} · packs catalog at {}",
        library.root().display(),
        pack_catalog_path.display()
    ));
    let (pack_catalog, registry, pack_status) = match PackCatalog::open(&pack_catalog_path) {
        Ok(catalog) => match build_registry(Some(&catalog)) {
            Ok(registry) => (Some(catalog), Arc::new(registry), None),
            Err(error) => (
                Some(catalog),
                Arc::new(world_builtins::registry()?),
                Some(format!("Installed Packs were not activated: {error}")),
            ),
        },
        Err(error) => (
            None,
            Arc::new(world_builtins::registry()?),
            Some(format!(
                "Could not open Installed Packs catalog {}: {error}",
                pack_catalog_path.display()
            )),
        ),
    };
    let (included_packs, included_status) = match included_packs::discover() {
        Ok(packs) => (packs, None),
        Err(error) => (
            Vec::new(),
            Some(format!("Could not locate included World Packs: {error}")),
        ),
    };
    let (documents, unreadable_documents, library_status) = match library.listing() {
        Ok(listing) => (listing.documents, listing.unreadable, None),
        Err(error) => (
            Vec::new(),
            Vec::new(),
            Some(format!("Could not read World Library: {error}")),
        ),
    };
    report_unreadable_documents(&unreadable_documents);

    diagnostics::record_environment(diagnostics::Environment {
        library_dir: Some(library.root().to_path_buf()),
        pack_catalog_path: Some(pack_catalog_path.clone()),
        included_packs: included_packs
            .iter()
            .map(|pack| format!("{} {}", pack.pack.id, pack.pack.version))
            .collect(),
    });
    diagnostics::info(format!(
        "{} saved World(s) · {} included Pack(s)",
        documents.len(),
        included_packs.len()
    ));

    let status = pack_status
        .or(library_status)
        .or(included_status)
        .map(HomeStatus::error);

    // Clicking the Dock icon after the last window was closed brings Home
    // back, the way a document-based Mac app behaves.
    application.on_reopen(|cx| {
        if cx.windows().is_empty() {
            if let Some(home) = cx.try_global::<HomeEntity>().map(|home| home.0.clone()) {
                open_home_window(home, cx);
            }
        }
    });

    application.run(move |cx: &mut App| {
        about::install(cx);
        text_field::bind_keys(cx);
        // A World's changes are written away from its turns; everything
        // handed over is on disk before the app goes (a window closing
        // writes its World's the same way, as its session is let go of).
        cx.on_app_quit(|_| {
            for error in world_library::flush_all_writes() {
                diagnostics::error(format!("saving as the app quits: {error}"));
            }
            async {}
        })
        .detach();
        let home = cx.new(|cx| {
            let world_search = cx.new(|cx| TextInput::new("Find a World…", cx));
            // Typing filters the list, so Home has to redraw as the field changes.
            cx.observe(&world_search, |_, _, cx| cx.notify()).detach();
            let mut home = WorldMachineHome {
                registry,
                library,
                pack_catalog,
                pack_catalog_path,
                documents,
                selected_world_pack: None,
                included_packs,
                pending_pack_install: None,
                pending_start_after_install: None,
                ready_pack_to_create: None,
                probing_packs: Vec::new(),
                status,
                show_packs: false,
                available_update: None,
                unreadable_documents,
                renaming: None,
                pending_removal: None,
                card_menu: None,
                world_sort: WorldSort::Recent,
                world_search,
                step_aside_for_first_world: false,
            };
            home.start_system_open_listener(cx);
            home.activate_included_packs(cx);
            home.start_update_check(cx);
            home
        });
        about::install_home_actions(&home, cx);
        cx.set_global(HomeEntity(home.clone()));
        open_home_window(home, cx);
        cx.activate(true);
    });

    Ok(())
}

/// The one Home entity, kept alive across window closes so the library
/// listener and Pack activation state survive Cmd-W.
/// Turns on the gentle pointers from the record in the app's settings,
/// writing it back, off the window's thread, whenever it grows.
#[cfg(gui)]
fn install_pointers(saved: Option<&world_machine_desktop::app_settings::AppSettings>) {
    use world_machine_desktop::app_settings::{self, HintSettings};
    let hints = saved
        .map(|settings| settings.hints.clone())
        .unwrap_or_default();
    world_gpui::pointers::install(
        world_gpui::pointers::Record::from_keys(&hints.shown, &hints.used),
        |record| {
            let (shown, used) = record.keys();
            std::thread::spawn(move || {
                let saved = app_settings::application_support_root()
                    .and_then(|root| app_settings::save_hints(&root, HintSettings { shown, used }));
                if let Err(error) = saved {
                    diagnostics::error(format!("could not keep the hints record: {error}"));
                }
            });
        },
    );
}

#[cfg(gui)]
struct HomeEntity(Entity<WorldMachineHome>);

#[cfg(gui)]
impl Global for HomeEntity {}

/// Brings Home back when no World, strip or Home window is left open, so
/// closing the last of them never leaves the app with nothing on screen.
#[cfg(gui)]
fn restore_home_if_nothing_open(cx: &mut App) {
    let windows = cx.windows();
    let world_or_home_open = windows.iter().any(|window| {
        window.downcast::<WorldDocumentView>().is_some()
            || window.downcast::<WorldMachineHome>().is_some()
            || window.downcast::<world_gpui::strip::StripView>().is_some()
    });
    if !world_or_home_open {
        if let Some(home) = cx.try_global::<HomeEntity>().map(|home| home.0.clone()) {
            open_home_window(home, cx);
        }
    }
}

#[cfg(gui)]
fn open_home_window(home: Entity<WorldMachineHome>, cx: &mut App) {
    let bounds = remembered_window_bounds(RememberedWindow::Home, cx)
        .unwrap_or_else(|| Bounds::centered(None, size(px(760.0), px(760.0)), cx));
    if let Err(error) = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |window, _| {
            watch_appearance(window);
            home
        },
    ) {
        diagnostics::error(format!("could not open the Home window: {error}"));
    }
}

#[cfg(not(gui))]
fn main() {
    eprintln!(
        "world-machine-desktop has no window on this platform; on Linux, build it with --features linux-window"
    );
}
