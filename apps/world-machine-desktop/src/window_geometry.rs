//! Where the app's windows were: each window reports its rectangle as it
//! renders, and Home's background loop writes what changed a few times a
//! second (`Window State.json`), so the app opens where it was left.
//! Moved out of `main.rs` as it stood.

#[allow(unused_imports)]
use super::*;

/// 200 ms ticks between writes of changed window geometry.
pub(crate) const WINDOW_GEOMETRY_FLUSH_TICKS: u32 = 5;

/// Which window a remembered rectangle belongs to. World windows share one
/// entry: reopening a World puts it where the last World window was, which is
/// what "the app opens where I left it" means with several Worlds open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RememberedWindow {
    Home,
    World,
}

/// The latest geometry each window has reported, and what is already on disk.
/// Windows report while they render; the Home background loop writes the
/// difference a few times a second, so dragging a window is not a stream of
/// file writes.
pub(crate) struct WindowGeometry {
    home: Option<StoredWindowBounds>,
    world: Option<StoredWindowBounds>,
    saved_home: Option<StoredWindowBounds>,
    saved_world: Option<StoredWindowBounds>,
}

pub(crate) static WINDOW_GEOMETRY: Mutex<WindowGeometry> = Mutex::new(WindowGeometry {
    home: None,
    world: None,
    saved_home: None,
    saved_world: None,
});

pub(crate) fn stored_bounds(bounds: Bounds<gpui::Pixels>) -> StoredWindowBounds {
    StoredWindowBounds::new(
        f32::from(bounds.origin.x),
        f32::from(bounds.origin.y),
        f32::from(bounds.size.width),
        f32::from(bounds.size.height),
    )
}

pub(crate) fn restored_bounds(stored: StoredWindowBounds) -> Bounds<gpui::Pixels> {
    Bounds::new(
        point(px(stored.x), px(stored.y)),
        size(px(stored.width), px(stored.height)),
    )
}

/// The displays a window could be reopened onto right now.
pub(crate) fn display_bounds(cx: &App) -> Vec<StoredWindowBounds> {
    cx.displays()
        .into_iter()
        .map(|display| stored_bounds(display.bounds()))
        .collect()
}

/// Note where a window is now. Only an ordinary windowed rectangle is worth
/// remembering: a maximized or full-screen window should not reopen at the
/// size of somebody's screen.
pub(crate) fn remember_window_geometry(window: &Window, which: RememberedWindow) {
    let WindowBounds::Windowed(bounds) = window.window_bounds() else {
        return;
    };
    let stored = stored_bounds(bounds);
    if !stored.is_plausible() {
        return;
    }
    let Ok(mut geometry) = WINDOW_GEOMETRY.lock() else {
        return;
    };
    match which {
        RememberedWindow::Home => geometry.home = Some(stored),
        RememberedWindow::World => geometry.world = Some(stored),
    }
}

/// Where a window should open, or `None` for its default place.
pub(crate) fn remembered_window_bounds(
    which: RememberedWindow,
    cx: &App,
) -> Option<Bounds<gpui::Pixels>> {
    let geometry = WINDOW_GEOMETRY.lock().ok()?;
    let stored = match which {
        RememberedWindow::Home => geometry.home,
        RememberedWindow::World => geometry.world,
    };
    drop(geometry);
    StoredWindowBounds::restorable(stored, &display_bounds(cx)).map(restored_bounds)
}

/// Write any geometry that changed since the last write. Cheap and silent when
/// nothing moved, which is the common case.
pub(crate) fn flush_window_geometry() {
    let Ok(geometry) = WINDOW_GEOMETRY.lock() else {
        return;
    };
    if geometry.home == geometry.saved_home && geometry.world == geometry.saved_world {
        return;
    }
    let (home, world) = (geometry.home, geometry.world);
    drop(geometry);

    let Ok(root) = world_machine_desktop::app_settings::application_support_root() else {
        return;
    };
    let mut state = window_state::load(&root);
    state.home = home.or(state.home);
    state.world = world.or(state.world);
    match window_state::save(&root, &state) {
        Ok(()) => {
            if let Ok(mut geometry) = WINDOW_GEOMETRY.lock() {
                geometry.saved_home = home;
                geometry.saved_world = world;
            }
        }
        // Where the windows were is a convenience; failing to record it is a
        // line in the log, never something in front of somebody.
        Err(error) => diagnostics::error(format!("could not record window positions: {error}")),
    }
}

/// Seed the remembered geometry from disk at launch.
pub(crate) fn load_window_geometry() {
    let Ok(root) = world_machine_desktop::app_settings::application_support_root() else {
        return;
    };
    let state = window_state::load(&root);
    if let Ok(mut geometry) = WINDOW_GEOMETRY.lock() {
        geometry.home = state.home;
        geometry.world = state.world;
        geometry.saved_home = state.home;
        geometry.saved_world = state.world;
    }
}
