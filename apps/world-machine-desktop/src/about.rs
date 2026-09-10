//! The About window and the application menu.
//!
//! Both exist so a user can tell which build they run, find the log, and
//! report a problem without a Terminal: About shows the build identity, the
//! not-notarized status, and the paths in use; the Help menu and the About
//! buttons copy a diagnostics report, reveal the log folder, and open the
//! issue template or the install guide.

use gpui::{
    actions, div, prelude::*, px, size, App, AppContext, Bounds, ClipboardItem, Context, Entity,
    IntoElement, KeyBinding, Menu, MenuItem, Render, Styled, SystemMenuType, Window, WindowBounds,
    WindowOptions,
};

use crate::{build_info, diagnostics, WorldMachineHome};

actions!(
    world_machine,
    [
        About,
        Settings,
        Quit,
        CopyDiagnostics,
        OpenLogFolder,
        ReportProblem,
        InstallGuide,
        CheckForUpdates,
        // Standard app and window behaviour.
        HideApp,
        HideOthers,
        ShowAll,
        CloseWindow,
        MinimizeWindow,
        ZoomWindow,
        // File menu, handled by Home wherever it is.
        ImportWorld,
        InstallPack,
        RefreshLibrary,
        // World menu, handled by the frontmost World window and greyed out
        // elsewhere because the handlers live on that window's root.
        BranchWorld,
        WhatIf,
        SaveWorldAs,
        ReloadWorld,
        CompareWithParent,
        CompareSavedWorlds,
        ShowLineage,
        AnalyzeWorlds
    ]
);

/// Registers the global actions, the menu bar, and Cmd-Q. Call once after the
/// application starts and before the first window opens.
pub fn install(cx: &mut App) {
    cx.on_action(|_: &About, cx| open_about_window(cx));
    cx.on_action(|_: &Settings, cx| crate::settings::open(cx));
    cx.on_action(|_: &Quit, cx| {
        diagnostics::info("quit requested from the menu");
        cx.quit();
    });
    cx.on_action(|_: &CopyDiagnostics, cx| copy_diagnostics(cx));
    cx.on_action(|_: &OpenLogFolder, cx| open_log_folder(cx));
    cx.on_action(|_: &ReportProblem, cx| {
        diagnostics::info("opening the issue template");
        cx.open_url(&diagnostics::issue_url());
    });
    cx.on_action(|_: &InstallGuide, cx| cx.open_url(diagnostics::INSTALL_GUIDE_URL));
    cx.on_action(|_: &CheckForUpdates, cx| {
        diagnostics::info("opening the releases page");
        cx.open_url(diagnostics::RELEASES_URL);
    });

    cx.on_action(|_: &HideApp, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &CloseWindow, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
    });
    cx.on_action(|_: &MinimizeWindow, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.minimize_window());
        }
    });
    cx.on_action(|_: &ZoomWindow, cx| {
        if let Some(window) = cx.active_window() {
            let _ = window.update(cx, |_, window, _| window.zoom_window());
        }
    });

    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("alt-cmd-h", HideOthers, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-m", MinimizeWindow, None),
        // Cmd-, is Settings everywhere else on this platform.
        KeyBinding::new("cmd-,", Settings, None),
    ]);

    cx.set_menus([
        Menu::new("World Machine").items([
            MenuItem::action("About World Machine…", About),
            MenuItem::separator(),
            MenuItem::action("Settings…", Settings),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Hide World Machine", HideApp),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::action("Show All", ShowAll),
            MenuItem::separator(),
            MenuItem::action("Quit World Machine", Quit),
        ]),
        Menu::new("File").items([
            MenuItem::action("Import World…", ImportWorld),
            MenuItem::action("Install World Pack…", InstallPack),
            MenuItem::separator(),
            MenuItem::action("Refresh My Worlds", RefreshLibrary),
        ]),
        Menu::new("World").items([
            MenuItem::action("Branch This World", BranchWorld),
            MenuItem::action("What If…", WhatIf),
            MenuItem::separator(),
            MenuItem::action("Save As…", SaveWorldAs),
            MenuItem::action("Reload from Disk", ReloadWorld),
            MenuItem::separator(),
            MenuItem::action("Compare with Parent", CompareWithParent),
            MenuItem::action("Compare Saved Worlds…", CompareSavedWorlds),
            MenuItem::action("Lineage…", ShowLineage),
            MenuItem::separator(),
            MenuItem::action("Analyze Saved Worlds (Experimental)…", AnalyzeWorlds),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", MinimizeWindow),
            MenuItem::action("Zoom", ZoomWindow),
            MenuItem::separator(),
            MenuItem::action("Close Window", CloseWindow),
        ]),
        Menu::new("Help").items([
            MenuItem::action("Check for Updates…", CheckForUpdates),
            MenuItem::action("Install Guide", InstallGuide),
            MenuItem::action("Report a Problem…", ReportProblem),
            MenuItem::separator(),
            MenuItem::action("Copy Diagnostics", CopyDiagnostics),
            MenuItem::action("Show Log in Finder", OpenLogFolder),
        ]),
    ]);
}

/// Routes the File menu to Home. Registered globally so the items work from
/// any window; Home stays the owner of the library and the Pack catalog.
pub fn install_home_actions(home: &Entity<WorldMachineHome>, cx: &mut App) {
    let import = home.clone();
    cx.on_action(move |_: &ImportWorld, cx| {
        import.update(cx, |home, cx| home.import_world(cx));
    });
    let install = home.clone();
    cx.on_action(move |_: &InstallPack, cx| {
        install.update(cx, |home, cx| home.install_pack(cx));
    });
    let refresh = home.clone();
    cx.on_action(move |_: &RefreshLibrary, cx| {
        refresh.update(cx, |home, cx| home.refresh_from_menu(cx));
    });
}

fn copy_diagnostics(cx: &mut App) {
    let report = diagnostics::report();
    cx.write_to_clipboard(ClipboardItem::new_string(report));
    diagnostics::info("diagnostics report copied to the clipboard");
}

fn open_log_folder(cx: &mut App) {
    match diagnostics::log_path() {
        Some(path) => cx.reveal_path(path),
        None => {
            if let Some(dir) = diagnostics::log_dir() {
                cx.reveal_path(&dir);
            }
        }
    }
}

fn open_about_window(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(520.0), px(420.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |_, cx| cx.new(|_| AboutView { copied: false }),
    );
    if let Err(error) = opened {
        diagnostics::error(format!("could not open the About window: {error}"));
    }
}

struct AboutView {
    copied: bool,
}

impl Render for AboutView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_window_title("About World Machine");
        let environment = diagnostics::environment();
        let library = environment
            .library_dir
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let log = diagnostics::log_path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_string());

        let copy_label = if self.copied {
            "Copied"
        } else {
            "Copy Diagnostics"
        };

        div()
            .size_full()
            .bg(crate::theme_rgb(0xfcfcfa))
            .text_color(crate::theme_rgb(0x202020))
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .child(div().text_xl().child("World Machine"))
            .child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x666666))
                    .child("Persistent worlds that remember, evolve, and branch."),
            )
            .child(div().text_sm().child(build_info::display_label()))
            .child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x9b4a42))
                    .child("Not yet notarized by Apple: the first launch asks once."),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(diagnostics::host_description()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x666666))
                    .child(format!("Worlds: {library}"))
                    .child(format!("Log: {log}")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x8a8a82))
                    .child(
                        "No account, no telemetry. Your Worlds are files on this Mac; \
                         only the optional World voice and Analyst send anything out, \
                         and only once you turn them on.",
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .pt_2()
                    .child(about_button("about-copy-diagnostics", copy_label).on_click(
                        cx.listener(|this, _, _, cx| {
                            copy_diagnostics(cx);
                            this.copied = true;
                            cx.notify();
                        }),
                    ))
                    .child(
                        about_button("about-show-log", "Show Log in Finder")
                            .on_click(cx.listener(|_, _, _, cx| open_log_folder(cx))),
                    )
                    .child(
                        about_button("about-report-problem", "Report a Problem…").on_click(
                            cx.listener(|_, _, _, cx| {
                                diagnostics::info("opening the issue template");
                                cx.open_url(&diagnostics::issue_url());
                            }),
                        ),
                    )
                    .child(
                        about_button("about-install-guide", "Install Guide").on_click(
                            cx.listener(|_, _, _, cx| cx.open_url(diagnostics::INSTALL_GUIDE_URL)),
                        ),
                    ),
            )
    }
}

fn about_button(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .cursor_pointer()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(crate::theme_rgb(0xd9d9d3))
        .bg(crate::theme_rgb(0xffffff))
        .text_sm()
        .child(label)
}
