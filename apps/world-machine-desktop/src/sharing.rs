//! Sharing a World: its code, copied or saved as a `.worldcode` file, a
//! visit to a World someone shared, and a resident of a friend's World
//! come to stay in one of yours.
//!
//! A visit is a copy replayed from the code's recorded history. Its window
//! says so, offers nothing that would change it, refuses anything that
//! asks to, and is never saved: it has no file and no place in My Worlds.
//! A friend's resident is read from such a copy, as their World shows
//! them, and arrives in yours only as a guest your World checks and
//! records; nothing is written to the friend's World.

use super::*;
use gpui::ClipboardItem;
use world_library::{looks_like_world_code, write_world_code_file, WorldVisit, WORLD_CODE_SUFFIX};
use world_projection::Guest;

/// Who from a friend's World could come to visit: everyone living there,
/// each as their World draws them and saying the last thing they said
/// there, which they bring as their letter too. Read from the visit's copy
/// only.
pub(crate) fn friends_residents(visit: &WorldVisit) -> Vec<Guest> {
    let from = visit.display_name();
    Guest::residents(&visit.snapshot())
        .into_iter()
        .map(|mut guest| {
            guest.from = from.clone();
            guest.gift = format!("a postcard of {from}");
            if let Some(line) = &guest.line {
                guest.letter = line.clone();
            }
            guest
        })
        .collect()
}

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

    fn story(&mut self, request: world_gpui::StoryRequest) -> Option<world_gpui::StoryPage> {
        let page = self.visit.borrow().story(request).ok()??;
        Some(world_gpui::i18n::localize_story(page))
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

    /// World ▸ Invite a Guest: someone from another of the player's Worlds
    /// visits this one. The other World is only read from the library's
    /// listing; the visit is an intent this World checks like any other.
    pub(crate) fn invite_guest(&mut self, cx: &mut Context<Self>) {
        let guest = {
            let document = self.document.borrow();
            let own = document.session.document_id().cloned();
            match document.library.list() {
                Err(error) => Err(error.to_string()),
                Ok(worlds) => worlds
                    .iter()
                    .filter(|world| Some(&world.id) != own.as_ref())
                    .find_map(world_library::guest_from)
                    .ok_or_else(|| {
                        "Start another World first: a guest comes from one of your other Worlds"
                            .to_string()
                    }),
            }
        };
        match guest {
            Ok(guest) => self.host_guest(guest, cx),
            Err(error) => {
                self.status = Some(DocumentStatus::error(error));
                cx.notify();
            }
        }
    }

    /// World ▸ Invite a Friend's Resident…: a friend's World code, from the
    /// clipboard or a `.worldcode` file, is opened as a copy, and the
    /// player chooses who from there comes to stay.
    pub(crate) fn invite_friend(&mut self, cx: &mut Context<Self>) {
        let text = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap_or_default();
        if looks_like_world_code(&text) {
            let opened = WorldVisit::open_code(&text, &self.document.borrow().registry);
            self.choose_friend(opened, cx);
            return;
        }
        let picker = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(mut paths))) = picker.await else {
                return;
            };
            let Some(path) = paths.pop() else {
                return;
            };
            let _ = this.update(cx, |this, cx| {
                let opened = WorldVisit::open_file(&path, &this.document.borrow().registry);
                this.choose_friend(opened, cx);
            });
        })
        .detach();
    }

    fn choose_friend(
        &mut self,
        opened: Result<WorldVisit, world_library::WorldCodeError>,
        cx: &mut Context<Self>,
    ) {
        let visit = match opened {
            Ok(visit) => visit,
            Err(error) => {
                self.status = Some(DocumentStatus::error(error.to_string()));
                cx.notify();
                return;
            }
        };
        let residents = friends_residents(&visit);
        if residents.is_empty() {
            self.status = Some(DocumentStatus::error(format!(
                "Nobody lives in {} yet to come and visit",
                visit.display_name()
            )));
            cx.notify();
            return;
        }
        let document = self.document.clone();
        let world = visit.display_name();
        let bounds = Bounds::centered(None, size(px(440.0), px(560.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                watch_appearance(window);
                window.set_window_title(&format!("Who from {world} will visit?"));
                cx.new(|_| FriendPicker {
                    document,
                    world,
                    residents,
                })
            },
        );
        if let Err(error) = opened {
            self.status = Some(DocumentStatus::error(error.to_string()));
            cx.notify();
        }
    }

    /// A guest comes to stay: an intent this World checks like any other.
    pub(crate) fn host_guest(&mut self, guest: Guest, cx: &mut Context<Self>) {
        let result = {
            let mut document = self.document.borrow_mut();
            let library = std::sync::Arc::clone(&document.library);
            let registry = std::sync::Arc::clone(&document.registry);
            document
                .session
                .handle(
                    world_projection::ProjectionIntent::Host(guest.clone()),
                    &registry,
                    &library,
                )
                .map(|_| guest)
                .map_err(|error| error.to_string())
        };
        self.status = Some(match result {
            Ok(guest) => {
                self.rebuild_projection(cx);
                DocumentStatus::success(format!(
                    "{} came over from {} with a letter",
                    guest.name, guest.from
                ))
            }
            Err(error) => DocumentStatus::error(error),
        });
        cx.notify();
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
                "There is no World code on the clipboard. Copy one (it starts with wm) and try again",
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

/// The window a friend's residents are chosen from: each drawn as their
/// World draws them, with their name and what they last said there.
pub(crate) struct FriendPicker {
    document: SharedDocument,
    world: String,
    residents: Vec<Guest>,
}

impl FriendPicker {
    /// Sends `guest` to the World this picker was opened for, through its
    /// window so it shows them at once, and closes.
    fn choose(&mut self, guest: Guest, window: &mut Window, cx: &mut Context<Self>) {
        let document = self.document.clone();
        let world = cx.windows().into_iter().find_map(|handle| {
            let world = handle.downcast::<WorldDocumentView>()?;
            world
                .read(cx)
                .is_ok_and(|view| Rc::ptr_eq(&view.document, &document))
                .then_some(world)
        });
        if let Some(world) = world {
            let _ = world.update(cx, move |view, window, cx| {
                view.host_guest(guest, cx);
                window.activate_window();
            });
        }
        window.remove_window();
    }
}

/// A resident's drawing, in their own look, waving.
fn resident_portrait(guest: &Guest) -> impl IntoElement {
    let drawing = guest
        .drawing
        .clone()
        .unwrap_or_else(|| world_projection::person_base("guest"));
    let inks =
        world_gpui::art::Inks::of_person(&world_gpui::art::Figure::of(&guest.name, guest.look));
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let (x, y) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            let tall = h * 0.9;
            world_gpui::art::paint_drawing(
                window,
                x + w / 2.0,
                y + h - 2.0,
                (tall * drawing.aspect).min(w),
                tall,
                &drawing,
                &inks,
                world_projection::Stance::Waving,
                world_projection::Mood::Happy,
                0.0,
                0.0,
                1.0,
            );
        },
    )
    .w(px(48.0))
    .h(px(64.0))
    .flex_shrink_0()
}

impl Render for FriendPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        let rows = self
            .residents
            .iter()
            .enumerate()
            .map(|(index, guest)| {
                let chosen = guest.clone();
                div()
                    .id(("friend-resident", index))
                    .role(gpui::Role::Button)
                    .aria_label(format!("{} from {}", guest.name, self.world))
                    .flex()
                    .gap_3()
                    .items_center()
                    .p_2()
                    .rounded_lg()
                    .cursor_pointer()
                    .hover(|style| style.bg(ui::color(tokens::ROW_HOVER)))
                    .child(resident_portrait(guest))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(guest.name.clone()),
                            )
                            .children(guest.line.clone().map(|line| {
                                div()
                                    .text_xs()
                                    .text_color(ui::color(tokens::TEXT_SECONDARY))
                                    .child(format!("“{line}”"))
                            })),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.choose(chosen.clone(), window, cx)
                    }))
            })
            .collect::<Vec<_>>();
        div()
            .id("friend-picker")
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .child(
                div()
                    .text_base()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(format!("Who from {} will visit?", self.world)),
            )
            .child(ui::caption(
                "They stay a few days and bring a letter. Nothing is changed in their World.",
            ))
            .child(
                div()
                    .id("friend-residents")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(rows),
            )
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

    /// A friend's World code shows who lives there, each with their own
    /// drawing and line; one of them comes to stay in another World, stands
    /// on its canvas drawn as at home, and the friend's World is unchanged.
    #[test]
    fn a_friends_resident_comes_to_stay_and_their_world_is_untouched() {
        let (visit, registry) = played_visit();
        let before = visit.archive().clone();
        let residents = friends_residents(&visit);
        assert!(!residents.is_empty(), "somebody lives there");
        assert!(residents
            .iter()
            .all(|guest| guest.from == visit.display_name()));
        let drawn = residents
            .iter()
            .find(|guest| guest.drawing.is_some())
            .cloned()
            .expect("a resident drawn their World's way");

        let pack = registry.descriptors()[0].pack.id.clone();
        let mut home = registry.create(&pack).unwrap();
        // Somebody must live there to welcome them.
        let first = home.snapshot().commands[0].id.clone();
        home.handle(world_gpui::ProjectionIntent::InvokeCommand(first))
            .unwrap();
        let after = home
            .handle(world_gpui::ProjectionIntent::Host(drawn.clone()))
            .unwrap();
        let standing = after
            .canvas
            .items
            .iter()
            .find(|item| item.label == drawn.name && item.detail.contains("Visiting"))
            .expect("the guest stands on the canvas");
        let drawing = after.drawing_of(standing).expect("their own drawing");
        assert_eq!(drawing.parts, drawn.travelling_drawing().unwrap().parts);
        assert_eq!(
            visit.archive(),
            &before,
            "nothing is written to the friend's World"
        );
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
