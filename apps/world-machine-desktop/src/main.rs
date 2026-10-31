#![forbid(unsafe_code)]
// A release build on Windows is a windowed program: no console window opens
// beside it. Debug builds keep the console for what the app prints.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(gui)]
mod about;
#[cfg(gui)]
mod app_menu;
#[cfg(gui)]
mod build_info;
#[cfg(gui)]
mod crash_window;
#[cfg(gui)]
mod diagnostics;
#[cfg(gui)]
mod included_packs;
#[cfg(gui)]
mod notes_window;
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
mod document_window;
#[cfg(gui)]
mod home;
#[cfg(gui)]
mod host_controller;
#[cfg(gui)]
mod library_setup;
#[cfg(gui)]
mod window_geometry;
#[cfg(gui)]
mod world_files;
#[cfg(gui)]
mod world_voice;
#[cfg(gui)]
use document_window::*;
#[cfg(gui)]
use home::*;
#[cfg(gui)]
use host_controller::*;
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
    WorldLibrary, WORLD_DOCUMENT_SUFFIX,
};
#[cfg(gui)]
use world_machine_desktop::ambience;
#[cfg(gui)]
use world_machine_desktop::demo;
#[cfg(gui)]
use world_machine_desktop::shell;
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

#[cfg(all(test, gui))]
mod file_type_tests;
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
        ambience::set_master(settings.master_level());
    }
    let reduce_motion = world_machine_desktop::display::reduce_motion(saved.as_ref());
    // A crash report the last run left, offered once unless the player
    // chose never to be asked. Nothing is sent from here.
    let crash_reports = crash_window::pending_offer(saved.as_ref());
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
                view: HomeView::Title,
                title_focus: shell::TitleChoice::Continue,
                menu_open: false,
                menu_focus: 0,
                focus: cx.focus_handle(),
                new_when_ready: false,
                showcase: None,
            };
            home.start_system_open_listener(cx);
            home.activate_included_packs(cx);
            home.start_update_check(cx);
            home
        });
        about::install_home_actions(&home, cx);
        cx.set_global(HomeEntity(home.clone()));
        open_home_window(home, cx);
        cx.set_reduce_motion(reduce_motion);
        crash_window::offer(crash_reports, cx);
        cx.activate(true);
    });

    Ok(())
}

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

/// The one Home entity, kept alive across window closes so the library
/// listener and Pack activation state survive Cmd-W.
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
