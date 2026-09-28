//! Sharing a World: its code, copied or saved as a `.worldcode` file, and a
//! visit to a World someone shared.
//!
//! A visit is a copy replayed from the code's recorded history. Its window
//! says so, offers nothing that would change it, refuses anything that
//! asks to, and is never saved: it has no file and no place in My Worlds.

use super::*;
use gpui::ClipboardItem;
use world_library::{looks_like_world_code, write_world_code_file, WorldVisit, WORLD_CODE_SUFFIX};

/// What a visit's window says it is, beside the World's name.
pub(crate) const VISITING_LABEL: &str = "Visiting — this is a copy";

/// A visit's World as its window shows it: everything a player could do
/// to change it is taken away, so the window offers no card, no hands and
/// no one to talk to in their own words. What people said, and what they
/// answer when asked, is still there to read.
pub(crate) fn visit_snapshot(
    mut snapshot: world_gpui::ProjectionSnapshot,
) -> world_gpui::ProjectionSnapshot {
    snapshot.commands.clear();
    snapshot.capabilities.fork = false;
    snapshot.capabilities.background = false;
    snapshot.capabilities.talk = false;
    for talk in &mut snapshot.talks {
        talk.asks_for = None;
    }
    snapshot
}

/// The window's side of a visit: it reads the copy and is refused if it
/// ever asks to change it.
pub(crate) struct VisitController {
    visit: Rc<RefCell<WorldVisit>>,
}

impl world_gpui::ProjectionController for VisitController {
    fn snapshot(&self) -> world_gpui::ProjectionSnapshot {
        world_gpui::i18n::localize(visit_snapshot(self.visit.borrow().snapshot()))
    }

    fn handle(
        &mut self,
        intent: world_gpui::ProjectionIntent,
    ) -> Result<world_gpui::ProjectionSnapshot, String> {
        self.visit
            .borrow_mut()
            .handle(intent)
            .map_err(|error| error.to_string())
    }

    fn cue(&mut self, cue: world_gpui::Cue) {
        ambience::player::cue(cue);
    }
}

/// The name a World's code is offered to be saved under.
pub(crate) fn suggested_code_file_name(name: &str) -> String {
    let stem = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '·' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    let stem = stem.split_whitespace().collect::<Vec<_>>().join(" ");
    let stem = if stem.is_empty() {
        "World".into()
    } else {
        stem
    };
    format!("{stem}{WORLD_CODE_SUFFIX}")
}

/// A save dialog's answer with the `.worldcode` ending it may have lost.
pub(crate) fn canonical_code_path(mut path: PathBuf) -> PathBuf {
    let has_suffix = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(WORLD_CODE_SUFFIX));
    if !has_suffix {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("World")
            .to_string();
        path.set_file_name(format!("{name}{WORLD_CODE_SUFFIX}"));
    }
    path
}

pub(crate) fn is_world_code_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(WORLD_CODE_SUFFIX))
}

impl WorldDocumentView {
    fn world_code(&self) -> Result<String, String> {
        self.document
            .borrow()
            .session
            .world_code()
            .map_err(|error| error.to_string())
    }

    /// World ▸ Copy World Code: the code on the clipboard, for a message.
    pub(crate) fn copy_world_code(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match self.world_code() {
            Ok(code) => {
                let length = code.len();
                cx.write_to_clipboard(ClipboardItem::new_string(code));
                DocumentStatus::success(format!(
                    "World code copied ({length} characters). Anyone can open it as a visit"
                ))
            }
            Err(error) => DocumentStatus::error(error),
        });
        cx.notify();
    }

    /// World ▸ Save World Code…: the code as a `.worldcode` file.
    pub(crate) fn save_world_code(&mut self, cx: &mut Context<Self>) {
        let code = match self.world_code() {
            Ok(code) => code,
            Err(error) => {
                self.status = Some(DocumentStatus::error(error));
                cx.notify();
                return;
            }
        };
        let suggested = suggested_code_file_name(&self.document_name);
        let dialog = cx.prompt_for_new_path(&PathBuf::default(), Some(&suggested));
        cx.spawn(async move |this, cx| {
            let status = match dialog.await {
                Ok(Ok(Some(path))) => {
                    let path = canonical_code_path(path);
                    match write_world_code_file(&path, &code) {
                        Ok(()) => DocumentStatus::success(format!(
                            "Saved the World code as {}",
                            path.file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or_default()
                        )),
                        Err(error) => {
                            DocumentStatus::error(format!("Could not save the World code: {error}"))
                        }
                    }
                }
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    DocumentStatus::error(format!("Could not open the Save dialog: {error}"))
                }
                Err(error) => {
                    DocumentStatus::error(format!("The Save dialog was interrupted: {error}"))
                }
            };
            let _ = this.update(cx, |this, cx| {
                this.status = Some(status);
                cx.notify();
            });
        })
        .detach();
    }
}

impl WorldMachineHome {
    /// File ▸ Paste World Code: opens the code on the clipboard as a visit.
    pub(crate) fn paste_world_code(&mut self, cx: &mut Context<Self>) {
        let text = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap_or_default();
        if !looks_like_world_code(&text) {
            self.status = Some(HomeStatus::error(
                "There is no World code on the clipboard. Copy one (it starts with wm1:) and try again",
            ));
            cx.notify();
            return;
        }
        let opened = WorldVisit::open_code(&text, &self.registry);
        self.open_visit_result(opened, cx);
    }

    /// File ▸ Open World Code…: opens a `.worldcode` file as a visit.
    pub(crate) fn open_world_code_file(&mut self, cx: &mut Context<Self>) {
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Visit".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = picker.await else {
                return;
            };
            let Some(path) = paths.pop() else {
                return;
            };
            let _ = this.update(cx, |this, cx| this.visit_path(&path, cx));
        })
        .detach();
    }

    /// Opens a `.worldcode` file as a visit. The file is only read.
    pub(crate) fn visit_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        let opened = WorldVisit::open_file(path, &self.registry);
        self.open_visit_result(opened, cx);
    }

    fn open_visit_result(
        &mut self,
        opened: Result<WorldVisit, world_library::WorldCodeError>,
        cx: &mut Context<Self>,
    ) {
        self.status = match opened {
            Ok(visit) => open_visit_window(visit, cx)
                .err()
                .map(|error| HomeStatus::error(format!("Could not open the visit: {error}"))),
            Err(error) => Some(HomeStatus::error(error.to_string())),
        };
        cx.notify();
    }
}

fn open_visit_window(visit: WorldVisit, cx: &mut App) -> Result<(), String> {
    let bounds = remembered_window_bounds(RememberedWindow::World, cx)
        .unwrap_or_else(|| Bounds::centered(None, size(px(1100.0), px(900.0)), cx));
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |window, cx| {
            watch_appearance(window);
            cx.new(|cx| WorldVisitView::new(visit, cx))
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

/// The window of a visit: the World, under a bar that names it and says
/// plainly that it is a copy.
pub(crate) struct WorldVisitView {
    name: String,
    visit: Rc<RefCell<WorldVisit>>,
    projection: Entity<world_gpui::ProjectionView>,
    status: Option<DocumentStatus>,
}

impl WorldVisitView {
    fn new(visit: WorldVisit, cx: &mut Context<Self>) -> Self {
        let name = visit.display_name();
        let visit = Rc::new(RefCell::new(visit));
        let controller = VisitController {
            visit: Rc::clone(&visit),
        };
        let projection =
            cx.new(|_| world_gpui::ProjectionView::controlled(controller).without_header());
        cx.observe(&projection, |_, _, cx| cx.notify()).detach();
        let sound_owner = cx.entity_id().as_u64();
        cx.on_release(move |_, cx| {
            ambience::player::release(sound_owner);
            // Closing the last window brings Home back.
            cx.defer(|cx| {
                if cx.windows().is_empty() {
                    if let Some(home) = cx.try_global::<HomeEntity>().map(|home| home.0.clone()) {
                        open_home_window(home, cx);
                    }
                }
            });
        })
        .detach();
        Self {
            name,
            visit,
            projection,
            status: None,
        }
    }

    /// A visit can be passed on: its code is the code it came from.
    fn copy_world_code(&mut self, cx: &mut Context<Self>) {
        self.status = Some(match self.visit.borrow().world_code() {
            Ok(code) => {
                cx.write_to_clipboard(ClipboardItem::new_string(code));
                DocumentStatus::success("World code copied")
            }
            Err(error) => DocumentStatus::error(error.to_string()),
        });
        cx.notify();
    }
}

impl Render for WorldVisitView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        window.set_window_title(&format!("{} (visit) — World Machine", self.name));
        let identity = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .gap_3()
            .items_center()
            .overflow_hidden()
            .child(
                div()
                    .flex_shrink_0()
                    .text_base()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .truncate()
                    .child(self.name.clone()),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .px_2()
                    .py(px(2.0))
                    .rounded_md()
                    .bg(ui::color(tokens::WARNING_SOFT))
                    .text_xs()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(ui::color(tokens::WARNING))
                    .child(ui::t(VISITING_LABEL)),
            );
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
        div()
            .size_full()
            .flex()
            .flex_col()
            .on_action(
                cx.listener(|this, _: &about::CopyWorldCode, _, cx| this.copy_world_code(cx)),
            )
            .child(chrome)
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .w_full()
                    .overflow_hidden()
                    .child(self.projection.clone()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_gpui::ProjectionController;

    fn played_visit() -> (WorldVisit, world_host::WorldRegistry) {
        let registry = world_builtins::registry().unwrap();
        let pack = registry.descriptors()[0].pack.id.clone();
        let mut session = registry.create(&pack).unwrap();
        for _ in 0..8 {
            let snapshot = session.snapshot();
            let Some(pick) = snapshot
                .commands
                .iter()
                .find(|command| command.unavailable.is_none())
                .map(|command| command.id.clone())
            else {
                break;
            };
            session
                .handle(world_gpui::ProjectionIntent::InvokeCommand(pick))
                .unwrap();
        }
        let archive = session.archive().unwrap().unwrap();
        let code = world_library::world_code_for_archive(archive, None).unwrap();
        (WorldVisit::open_code(&code, &registry).unwrap(), registry)
    }

    #[test]
    fn a_visit_window_offers_nothing_that_changes_the_world() {
        let (visit, _registry) = played_visit();
        let original = visit.snapshot();
        assert!(
            !original.commands.is_empty(),
            "the World itself offers choices"
        );
        let mut controller = VisitController {
            visit: Rc::new(RefCell::new(visit)),
        };
        let shown = controller.snapshot();
        assert!(shown.commands.is_empty(), "the visit offers none");
        assert!(shown.deeds().next().is_none(), "and nothing to build");
        assert!(!shown.capabilities.talk && !shown.capabilities.background);
        // Asked anyway, it refuses in words, and nothing moves.
        for command in &original.commands {
            let refused = controller
                .handle(world_gpui::ProjectionIntent::InvokeCommand(
                    command.id.clone(),
                ))
                .unwrap_err();
            assert!(refused.contains("visit"), "{refused}");
        }
        assert_eq!(controller.visit.borrow().snapshot(), original);
    }

    #[test]
    fn a_code_file_is_named_after_its_world() {
        assert_eq!(
            suggested_code_file_name("Harbor Town"),
            "Harbor Town.worldcode"
        );
        assert_eq!(
            suggested_code_file_name(" Ares / Ice: Colony? "),
            "Ares - Ice- Colony-.worldcode"
        );
        assert_eq!(suggested_code_file_name("  "), "World.worldcode");
        assert_eq!(
            canonical_code_path(PathBuf::from("/tmp/Harbor")),
            PathBuf::from("/tmp/Harbor.worldcode")
        );
        assert_eq!(
            canonical_code_path(PathBuf::from("/tmp/Harbor.worldcode")),
            PathBuf::from("/tmp/Harbor.worldcode")
        );
        assert!(is_world_code_file(Path::new("/tmp/a.worldcode")));
        assert!(!is_world_code_file(Path::new("/tmp/a.world")));
    }
}
