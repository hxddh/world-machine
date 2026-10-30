//! Home's title screen: the town at dusk, drawn by the World's own
//! renderer from the World Continue would open (or, before there is one,
//! from a new World of the featured Pack, made in memory and never saved),
//! the game's one-line promise, and Continue, New and Settings, walked
//! with the arrows and chosen with Enter. My Worlds and the rest are one
//! step away. The in-window app menu opens from its top right on either
//! page.

#[allow(unused_imports)]
use super::*;
use shell::{MenuItem, MenuPlace, TitleChoice};

/// Which page Home shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HomeView {
    Title,
    Library,
}

/// What the title screen draws its town from: a World's cast, colours,
/// marks and drawings, as a World's cover on Home has them.
pub(crate) struct Showcase {
    pub(crate) scenery: Option<world_projection::Scenery>,
    pub(crate) marks: Vec<world_projection::MarkShape>,
    pub(crate) cast: Vec<world_projection::CanvasItem>,
    pub(crate) drawings: Vec<world_projection::Drawing>,
}

impl Showcase {
    /// A World's town as its file last described it, if it has people and
    /// buildings to draw.
    fn of_document(document: &WorldDocumentSummary) -> Option<Self> {
        (!document.display_cast.is_empty()).then(|| Self {
            scenery: document.display_scenery,
            marks: document.display_marks.clone(),
            cast: document.display_cast.clone(),
            drawings: document.display_drawings.clone(),
        })
    }

    /// A new World of the first built-in World (the featured one), made
    /// in memory: the same Pack and the same renderer as a
    /// real one, never saved and never shown as a World of the player's.
    fn fresh() -> Option<Self> {
        let registry = world_builtins::registry().ok()?;
        let pack = registry.descriptors().first()?.pack.id.clone();
        let snapshot = registry.create(&pack).ok()?.snapshot();
        Some(Self {
            scenery: snapshot.scenery,
            marks: snapshot
                .canvas
                .marks
                .iter()
                .map(|mark| mark.shape)
                .collect(),
            cast: snapshot.canvas.items.clone(),
            drawings: snapshot.drawings.clone(),
        })
    }
}

impl WorldMachineHome {
    /// The World Continue opens: the most recently played World that has
    /// begun (the Library lists the most recent first).
    pub(crate) fn continue_world(&self) -> Option<&WorldDocumentSummary> {
        self.documents.iter().find(|document| has_begun(document))
    }

    /// The title screen's choices now.
    pub(crate) fn title_choices(&self) -> Vec<TitleChoice> {
        shell::title_choices(self.continue_world().is_some())
    }

    /// The choice the keyboard is on: the one last moved to, or the first
    /// when that one is not offered (New, on a first launch).
    fn focused_choice(&self, choices: &[TitleChoice]) -> TitleChoice {
        if choices.contains(&self.title_focus) {
            self.title_focus
        } else {
            choices[0]
        }
    }

    /// The included Pack Home features, which New starts a World of.
    pub(crate) fn featured_pack(&self) -> Option<WorldPackRef> {
        self.included_packs
            .iter()
            .find(|pack| pack.featured)
            .filter(|pack| !demo::ENABLED || demo::offers_pack(&pack.pack.id))
            .map(|pack| pack.pack.clone())
    }

    /// New: a World of the featured Pack, or of the first World the app
    /// can start when none is featured. While the featured Pack is still
    /// getting ready on a first launch, its World starts the moment it is.
    pub(crate) fn new_world(&mut self, cx: &mut Context<Self>) {
        let first = self.continue_world().is_none();
        if let Some(pack) = self.featured_pack() {
            if self.is_pack_probing(&pack) {
                self.new_when_ready = true;
                self.step_aside_for_first_world |= first;
                let title = self.included_pack_title(&pack).unwrap_or("World");
                self.status = Some(HomeStatus::info(format!(
                    "Preparing {title} for its first launch…"
                )));
                cx.notify();
                return;
            }
        }
        let pack_id = self
            .featured_pack()
            .map(|pack| pack.id)
            .filter(|id| self.registry.descriptor(id).is_some())
            .or_else(|| {
                self.registry
                    .descriptors()
                    .into_iter()
                    .map(|descriptor| descriptor.pack.id.clone())
                    .find(|id| !demo::ENABLED || demo::offers_pack(id))
            });
        let Some(pack_id) = pack_id else {
            self.view = HomeView::Library;
            cx.notify();
            return;
        };
        self.step_aside_for_first_world |= first;
        self.create_world(pack_id, cx);
    }

    /// Back to the title screen, with the keyboard on it.
    pub(crate) fn show_title(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.view = HomeView::Title;
        self.menu_open = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn choose(&mut self, choice: TitleChoice, window: &mut Window, cx: &mut Context<Self>) {
        self.title_focus = choice;
        match choice {
            TitleChoice::Continue => {
                if let Some(id) = self.continue_world().map(|document| document.id.clone()) {
                    self.open_document(id, cx);
                }
            }
            TitleChoice::New => self.new_world(cx),
            TitleChoice::Settings => window.dispatch_action(Box::new(about::Settings), cx),
            TitleChoice::MyWorlds => {
                self.view = HomeView::Library;
                window.focus(&self.focus, cx);
            }
        }
        cx.notify();
    }

    /// The app menu's items on Home.
    fn menu_items(&self) -> Vec<MenuItem> {
        shell::app_menu(MenuPlace::Home {
            has_world: self.continue_world().is_some(),
        })
    }

    fn toggle_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.menu_open = !self.menu_open;
        self.menu_focus = 0;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn pick(&mut self, item: MenuItem, window: &mut Window, cx: &mut Context<Self>) {
        self.menu_open = false;
        if !app_menu::run_app_item(item, window, cx) {
            self.save_continue_copy(cx);
        }
        cx.notify();
    }

    /// Save As on Home: a copy of the World Continue would open, the way
    /// its card's Export does.
    fn save_continue_copy(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.continue_world().map(|document| document.id.clone()) {
            self.export_document(id, cx);
        }
    }

    /// The arrows and Enter on the title screen and in the menu; Esc puts
    /// the menu away, or goes back from My Worlds to the title screen.
    fn on_key(&mut self, event: &gpui::KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let modifiers = keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt {
            return;
        }
        let key = keystroke.key.as_str();
        if self.menu_open {
            let items = self.menu_items();
            if key == "escape" {
                self.menu_open = false;
            } else if let Some(step) = shell::step_for_key(key, modifiers.shift) {
                self.menu_focus = shell::step(items.len(), self.menu_focus, step);
            } else if shell::chooses(key) {
                if let Some(item) = items.get(self.menu_focus).copied() {
                    self.pick(item, window, cx);
                }
            } else {
                return;
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        match self.view {
            HomeView::Title => {
                let choices = self.title_choices();
                let at = choices
                    .iter()
                    .position(|choice| *choice == self.focused_choice(&choices))
                    .unwrap_or(0);
                if let Some(step) = shell::step_for_key(key, modifiers.shift) {
                    self.title_focus = choices[shell::step(choices.len(), at, step)];
                } else if shell::chooses(key) {
                    self.choose(choices[at], window, cx);
                } else {
                    return;
                }
                cx.stop_propagation();
                cx.notify();
            }
            // Only where nothing else holds the keyboard: Esc in Find a
            // World is the field's own.
            HomeView::Library if key == "escape" && self.focus.is_focused(window) => {
                self.show_title(window, cx);
            }
            HomeView::Library => {}
        }
    }

    /// The town at dusk behind the title screen.
    fn backdrop(&mut self) -> gpui::AnyElement {
        let own = self.continue_world().and_then(Showcase::of_document);
        let town = match own {
            Some(own) => Some(Rc::new(own)),
            None => self
                .showcase
                .get_or_insert_with(|| Showcase::fresh().map(Rc::new))
                .clone(),
        };
        match town {
            Some(town) => world_gpui::diorama::cover_in(
                town.scenery,
                &town.marks,
                town.cast.clone(),
                town.drawings.clone(),
                world_gpui::scene::Daylight::Dusk,
            )
            .size_full()
            .into_any_element(),
            None => ui::cover(demo::HOME_TITLE, "title")
                .size_full()
                .into_any_element(),
        }
    }

    fn title_page(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let choices = self.title_choices();
        let focused = self.focused_choice(&choices);
        let continuing = self.continue_world().map(|document| {
            let pack_title = self
                .registry
                .descriptor_for(&document.pack)
                .map(|descriptor| descriptor.title.clone())
                .unwrap_or_else(|| document.pack.id.clone());
            let title = world_summary_title(document, &pack_title);
            let meta = world_card_meta(
                &title,
                &pack_title,
                document.world_time,
                document.display_calendar.as_ref(),
            );
            format!("{} · {meta}", ui::t(title))
        });
        let mut buttons = div()
            .id("title-choices")
            .role(gpui::Role::Menu)
            .aria_label(ui::t(shell::PROMISE))
            .w(px(300.0))
            .flex()
            .flex_col()
            .gap_2();
        for choice in choices {
            let is_focused = choice == focused;
            let label = ui::t(choice.label());
            let mut button = div()
                .id(choice.id())
                .role(gpui::Role::Button)
                .aria_label(label.clone())
                .w_full()
                .px_4()
                .py_2()
                .rounded_lg()
                .border_2()
                .cursor_pointer()
                .flex()
                .flex_col()
                .items_center()
                .text_base()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .map(|button| {
                    if is_focused {
                        // The focus is plain to see: the accent, ringed.
                        button
                            .bg(ui::color(tokens::ACCENT))
                            .text_color(ui::color(tokens::ON_ACCENT))
                            .border_color(ui::color(tokens::ON_ACCENT))
                            .shadow_lg()
                    } else {
                        button
                            .bg(gpui::Hsla::from(ui::color(tokens::SURFACE)).opacity(0.9))
                            .text_color(ui::color(tokens::TEXT))
                            .border_color(gpui::transparent_black())
                            .hover(|style| style.bg(ui::color(tokens::SURFACE)))
                    }
                })
                .child(label)
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    if *hovered && this.title_focus != choice {
                        this.title_focus = choice;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(move |this, _, window, cx| this.choose(choice, window, cx)));
            if let (TitleChoice::Continue, Some(line)) = (choice, continuing.clone()) {
                button = button.child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::NORMAL)
                        .truncate()
                        .child(line),
                );
            }
            buttons = buttons.child(button);
        }

        div()
            .id("title-screen")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(ui::color(tokens::WINDOW))
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .child(self.backdrop()),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    // The name and the promise in the sky, the choices on
                    // the shore, and the town between them.
                    .justify_between()
                    .pt(px(88.0))
                    .pb(px(84.0))
                    .child(
                        div()
                            .px_6()
                            .py_3()
                            .rounded_xl()
                            .bg(gpui::hsla(0.62, 0.35, 0.12, 0.55))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_3xl()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(gpui::hsla(0.11, 0.6, 0.94, 1.0))
                                    .child(ui::t(demo::HOME_TITLE)),
                            )
                            .child(
                                div()
                                    .text_base()
                                    .text_color(gpui::hsla(0.11, 0.45, 0.86, 1.0))
                                    .child(ui::t(shell::PROMISE)),
                            ),
                    )
                    .child(buttons),
            )
            .into_any_element()
    }
}

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
        // Nothing holds the keyboard (a window just opened, a field
        // closed): Home takes it, so the arrows always reach the choices.
        if window.focused(cx).is_none() {
            window.focus(&self.focus, cx);
        }

        let page = match self.view {
            HomeView::Title => self.title_page(cx),
            HomeView::Library => self.library_page(cx).into_any_element(),
        };
        let menu_open = self.menu_open;
        let menu = menu_open.then(|| {
            let pick: app_menu::Pick<Self> =
                Rc::new(|this: &mut Self, item, window, cx| this.pick(item, window, cx));
            div()
                .absolute()
                .top(px(52.0))
                .right(px(16.0))
                .child(app_menu::menu_list(
                    &self.menu_items(),
                    Some(self.menu_focus),
                    pick,
                    cx,
                ))
        });
        div()
            .id("world-machine-home")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            // The menu bar's Save As on Home saves a copy of the World
            // Continue would open, as the in-window menu's does.
            .on_action(
                cx.listener(|this, _: &about::SaveWorldAs, _, cx| this.save_continue_copy(cx)),
            )
            .relative()
            .size_full()
            .child(page)
            .children(self.status_overlay(cx))
            .child(
                div().absolute().top(px(12.0)).right(px(16.0)).child(
                    app_menu::menu_button("home-menu", menu_open)
                        .on_click(cx.listener(|this, _, window, cx| this.toggle_menu(window, cx))),
                ),
            )
            .children(menu)
    }
}
