//! The app's shell, as a plan the windows draw: the title screen's
//! choices and how the keyboard moves between them, and the in-window app
//! menu's items, with names that suit the computer the app runs on.
//!
//! Pure presentation: nothing here reads or changes a World. The windows
//! (`home/title.rs`, `app_menu.rs`) draw it and do what is chosen through
//! the app's existing actions.

/// The one line under the title: what the game promises.
pub const PROMISE: &str = "A small harbour that grows a day at a time";

/// A choice on the title screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TitleChoice {
    /// The most recently played World, opened again.
    Continue,
    /// A new World of the World the app features.
    New,
    /// The Settings window.
    Settings,
    /// Everything else on Home: My Worlds, Packs, imports, restores.
    MyWorlds,
}

impl TitleChoice {
    /// What the choice says, in English (the windows translate it).
    pub fn label(self) -> &'static str {
        match self {
            Self::Continue => "Continue",
            Self::New => "New World",
            Self::Settings => "Settings",
            Self::MyWorlds => "My Worlds",
        }
    }

    /// The element id the window gives it.
    pub fn id(self) -> &'static str {
        match self {
            Self::Continue => "title-continue",
            Self::New => "title-new",
            Self::Settings => "title-settings",
            Self::MyWorlds => "title-my-worlds",
        }
    }
}

/// The title screen's choices, in the order they are shown and the
/// keyboard walks them. With a World to go back to, Continue leads; on a
/// first launch (no World begun yet) New does, and Continue is not offered.
/// My Worlds is always one step away, last.
pub fn title_choices(can_continue: bool) -> Vec<TitleChoice> {
    let mut choices = Vec::with_capacity(4);
    if can_continue {
        choices.push(TitleChoice::Continue);
    }
    choices.extend([
        TitleChoice::New,
        TitleChoice::Settings,
        TitleChoice::MyWorlds,
    ]);
    choices
}

/// Which way a key moves the focus through a list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    Back,
    Forward,
}

/// The step a key asks for, by GPUI's key name: the arrows (up and left
/// go back), and Tab, forward (Shift-Tab back).
pub fn step_for_key(key: &str, shift: bool) -> Option<Step> {
    match key {
        "up" | "left" => Some(Step::Back),
        "down" | "right" => Some(Step::Forward),
        "tab" if shift => Some(Step::Back),
        "tab" => Some(Step::Forward),
        _ => None,
    }
}

/// Whether a key chooses the focused item.
pub fn chooses(key: &str) -> bool {
    matches!(key, "enter" | "space")
}

/// The focus after one step through `len` items from `at`, going round
/// at either end. An empty list has nothing to focus (0).
pub fn step(len: usize, at: usize, step: Step) -> usize {
    if len == 0 {
        return 0;
    }
    let at = at.min(len - 1);
    match step {
        Step::Back => (at + len - 1) % len,
        Step::Forward => (at + 1) % len,
    }
}

/// An item of the in-window app menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuItem {
    Settings,
    About,
    SaveAs,
    CopyWorldCode,
    SaveWorldCode,
    OpenWorldCode,
    PasteWorldCode,
    Help,
    Quit,
}

impl MenuItem {
    /// What the item says, in English (the windows translate it). The
    /// same words as the macOS menu bar's.
    pub fn label(self) -> &'static str {
        match self {
            Self::Settings => "Settings…",
            Self::About => "About World Machine…",
            Self::SaveAs => "Save As…",
            Self::CopyWorldCode => "Copy World Code",
            Self::SaveWorldCode => "Save World Code…",
            Self::OpenWorldCode => "Open World Code…",
            Self::PasteWorldCode => "Paste World Code",
            Self::Help => "Help",
            Self::Quit => "Quit World Machine",
        }
    }

    /// The element id the menu gives it.
    pub fn id(self) -> &'static str {
        match self {
            Self::Settings => "app-menu-settings",
            Self::About => "app-menu-about",
            Self::SaveAs => "app-menu-save-as",
            Self::CopyWorldCode => "app-menu-copy-world-code",
            Self::SaveWorldCode => "app-menu-save-world-code",
            Self::OpenWorldCode => "app-menu-open-world-code",
            Self::PasteWorldCode => "app-menu-paste-world-code",
            Self::Help => "app-menu-help",
            Self::Quit => "app-menu-quit",
        }
    }

    /// The group the item is drawn in; a line divides one group from the
    /// next.
    pub fn group(self) -> MenuGroup {
        match self {
            Self::Settings | Self::About => MenuGroup::App,
            Self::SaveAs => MenuGroup::World,
            Self::CopyWorldCode
            | Self::SaveWorldCode
            | Self::OpenWorldCode
            | Self::PasteWorldCode => MenuGroup::WorldCodes,
            Self::Help => MenuGroup::Help,
            Self::Quit => MenuGroup::Quit,
        }
    }

    /// Its keyboard shortcut as this computer's keyboard has it, where it
    /// has one: ⌘ on a Mac, Ctrl elsewhere.
    pub fn shortcut(self, mac: bool) -> Option<String> {
        let key = match self {
            Self::Settings => ",",
            Self::Quit => "Q",
            _ => return None,
        };
        Some(if mac {
            format!("⌘{key}")
        } else {
            format!("Ctrl+{key}")
        })
    }
}

/// The groups of the app menu, in order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuGroup {
    App,
    World,
    WorldCodes,
    Help,
    Quit,
}

impl MenuGroup {
    /// The small heading over the group, where it has one.
    pub fn heading(self) -> Option<&'static str> {
        match self {
            Self::WorldCodes => Some("World codes"),
            _ => None,
        }
    }
}

/// Where the app menu is opened from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuPlace {
    /// Home. `has_world`: there is a World to save a copy of (the one
    /// Continue opens).
    Home { has_world: bool },
    /// A World's own window.
    World,
}

/// The app menu's items from `place`: Settings, About, Save As, the World
/// codes, Help and Quit. A World's own code is copied or saved only from
/// its window; Home saves a copy of the World Continue would open, when
/// there is one.
pub fn app_menu(place: MenuPlace) -> Vec<MenuItem> {
    let mut items = vec![MenuItem::Settings, MenuItem::About];
    match place {
        MenuPlace::World => items.extend([
            MenuItem::SaveAs,
            MenuItem::CopyWorldCode,
            MenuItem::SaveWorldCode,
        ]),
        MenuPlace::Home { has_world: true } => items.push(MenuItem::SaveAs),
        MenuPlace::Home { has_world: false } => {}
    }
    items.extend([
        MenuItem::OpenWorldCode,
        MenuItem::PasteWorldCode,
        MenuItem::Help,
        MenuItem::Quit,
    ]);
    items
}

/// The operating systems whose words differ.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Os {
    Mac,
    Windows,
    Other,
}

impl Os {
    /// The one this app was built for.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Mac
        } else if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Other
        }
    }
}

/// What the button that shows the log is called: each system's own file
/// browser by name, or plainly the folder.
pub fn show_log_label(os: Os) -> &'static str {
    match os {
        Os::Mac => "Show Log in Finder",
        Os::Windows => "Show Log in File Explorer",
        Os::Other => "Show Log Folder",
    }
}

/// What About says of where Worlds are kept and what leaves the computer.
pub fn privacy_line(os: Os) -> &'static str {
    match os {
        Os::Mac => {
            "No account, no telemetry. Your Worlds are files on this Mac; only the optional World voice sends anything out, and only once you turn it on."
        }
        Os::Windows | Os::Other => {
            "No account, no telemetry. Your Worlds are files on this computer; only the optional World voice sends anything out, and only once you turn it on."
        }
    }
}

/// The short in-app Help page: how a World is played, a line each.
pub const HELP_LINES: &[&str] = &[
    "Each day you come back, your World lives one more day. It is saved as you play.",
    "Up and down choose a card; Enter takes it.",
    "Look closer with + and −, a pinch, or the scroll wheel.",
    "The drawer: story, letters, keepsakes and the book (⌘I)",
    "Esc puts away what is open.",
];

/// Every line the shell shows, in English, on `os`: for the translation
/// check.
pub fn every_line(os: Os) -> Vec<&'static str> {
    let mut lines = vec![
        PROMISE,
        "Menu",
        "Back",
        "How to play",
        "More help",
        show_log_label(os),
        privacy_line(os),
    ];
    for choice in title_choices(true) {
        lines.push(choice.label());
    }
    for item in app_menu(MenuPlace::World) {
        lines.push(item.label());
        if let Some(heading) = item.group().heading() {
            lines.push(heading);
        }
    }
    lines.extend(HELP_LINES);
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_first_launch_offers_new_first_and_no_continue() {
        let first = title_choices(false);
        assert_eq!(first[0], TitleChoice::New);
        assert!(!first.contains(&TitleChoice::Continue));
        assert_eq!(
            first,
            vec![
                TitleChoice::New,
                TitleChoice::Settings,
                TitleChoice::MyWorlds
            ]
        );
    }

    #[test]
    fn a_world_to_go_back_to_puts_continue_first_then_new_and_settings() {
        assert_eq!(
            title_choices(true),
            vec![
                TitleChoice::Continue,
                TitleChoice::New,
                TitleChoice::Settings,
                TitleChoice::MyWorlds
            ]
        );
    }

    #[test]
    fn the_arrows_walk_the_choices_and_go_round() {
        let len = title_choices(true).len();
        assert_eq!(step(len, 0, Step::Forward), 1);
        assert_eq!(step(len, 3, Step::Forward), 0);
        assert_eq!(step(len, 0, Step::Back), 3);
        assert_eq!(step(len, 2, Step::Back), 1);
        // A focus left past the end (a choice that went away) comes back.
        assert_eq!(step(3, 9, Step::Back), 1);
        assert_eq!(step(0, 0, Step::Forward), 0);
        assert_eq!(step_for_key("down", false), Some(Step::Forward));
        assert_eq!(step_for_key("right", false), Some(Step::Forward));
        assert_eq!(step_for_key("up", false), Some(Step::Back));
        assert_eq!(step_for_key("left", false), Some(Step::Back));
        assert_eq!(step_for_key("tab", false), Some(Step::Forward));
        assert_eq!(step_for_key("tab", true), Some(Step::Back));
        assert_eq!(step_for_key("a", false), None);
        assert!(chooses("enter"));
        assert!(chooses("space"));
        assert!(!chooses("escape"));
    }

    #[test]
    fn the_menu_has_the_whole_shell_in_a_world_window() {
        let world = app_menu(MenuPlace::World);
        for item in [
            MenuItem::Settings,
            MenuItem::About,
            MenuItem::SaveAs,
            MenuItem::CopyWorldCode,
            MenuItem::SaveWorldCode,
            MenuItem::OpenWorldCode,
            MenuItem::PasteWorldCode,
            MenuItem::Help,
            MenuItem::Quit,
        ] {
            assert!(world.contains(&item), "{item:?}");
        }
        assert_eq!(world.last(), Some(&MenuItem::Quit));
        // Groups never come back once left: each is drawn once.
        let groups = world.iter().map(|item| item.group()).collect::<Vec<_>>();
        let mut seen = Vec::new();
        for group in groups {
            if seen.last() != Some(&group) {
                assert!(!seen.contains(&group), "{group:?} drawn twice");
                seen.push(group);
            }
        }
    }

    #[test]
    fn home_saves_a_copy_only_of_a_world_it_has_and_never_a_world_code() {
        let empty = app_menu(MenuPlace::Home { has_world: false });
        assert!(!empty.contains(&MenuItem::SaveAs));
        let home = app_menu(MenuPlace::Home { has_world: true });
        assert!(home.contains(&MenuItem::SaveAs));
        for items in [empty, home] {
            assert!(!items.contains(&MenuItem::CopyWorldCode));
            assert!(!items.contains(&MenuItem::SaveWorldCode));
            for item in [
                MenuItem::Settings,
                MenuItem::About,
                MenuItem::OpenWorldCode,
                MenuItem::PasteWorldCode,
                MenuItem::Help,
                MenuItem::Quit,
            ] {
                assert!(items.contains(&item), "{item:?}");
            }
        }
    }

    #[test]
    fn no_mac_words_off_a_mac() {
        for os in [Os::Windows, Os::Other] {
            let mut shown = every_line(os)
                .into_iter()
                .filter(|line| !line.contains('⌘'))
                .map(str::to_string)
                .collect::<Vec<_>>();
            for item in app_menu(MenuPlace::World) {
                shown.extend(item.shortcut(false));
            }
            for line in shown {
                assert!(!line.contains('⌘'), "{os:?}: {line}");
                for word in line.split(|c: char| !c.is_alphanumeric()) {
                    assert!(
                        !["Finder", "Mac", "macOS", "Cmd", "Apple"].contains(&word),
                        "{os:?}: {line}"
                    );
                }
            }
        }
        assert_eq!(
            MenuItem::Settings.shortcut(false).as_deref(),
            Some("Ctrl+,")
        );
        assert_eq!(MenuItem::Quit.shortcut(true).as_deref(), Some("⌘Q"));
        assert_eq!(MenuItem::Help.shortcut(false), None);
    }

    /// Every word the shell shows is in the app's Chinese and Japanese.
    #[test]
    fn every_line_is_in_chinese_and_japanese() {
        for (name, app, packs) in [
            (
                "Chinese",
                world_gpui::i18n::APP_ZH_HANS,
                &world_builtins::ZH_HANS,
            ),
            ("Japanese", world_gpui::i18n::APP_JA, &world_builtins::JA),
        ] {
            let mut catalog = world_i18n::Catalog::parse(app);
            for pack in packs.iter() {
                catalog.extend(pack);
            }
            catalog.extend(app);
            for os in [Os::Mac, Os::Windows, Os::Other] {
                for line in every_line(os) {
                    let shown = catalog.translate(line);
                    assert!(
                        shown.as_deref().is_some_and(|shown| shown != line),
                        "{name}: {line:?} → {shown:?}"
                    );
                }
            }
        }
    }
}
