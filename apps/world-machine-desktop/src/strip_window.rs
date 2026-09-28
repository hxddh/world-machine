//! A World shown as a strip along the edge of a screen: opened from the
//! World menu, placed as the player chose (which edge, which display,
//! whether it stays in front), kept up to date with its World's window, and
//! opening that window again on a double-click.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    px, size, App, AppContext, Bounds, Entity, WindowBounds, WindowHandle, WindowKind,
    WindowOptions,
};
use world_gpui::strip::{self, Edge, StripView};
use world_gpui::ProjectionSnapshot;
use world_machine_desktop::app_settings::{self, StripSettings};

use crate::{
    about, diagnostics, remembered_window_bounds, watch_appearance, RememberedWindow,
    SharedDocument, WorldDocumentView,
};

/// A strip on screen, and the World it shows.
struct Strip {
    document: SharedDocument,
    window: WindowHandle<StripView>,
    view: Entity<StripView>,
}

thread_local! {
    static STRIPS: RefCell<Vec<Strip>> = const { RefCell::new(Vec::new()) };
}

/// Where strips go, as the player last chose; where they start if the
/// settings cannot be read.
pub fn placement() -> StripSettings {
    app_settings::application_support_root()
        .ok()
        .and_then(|root| app_settings::load(&root).ok())
        .map(|settings| settings.strip)
        .unwrap_or_default()
}

fn edge(placement: &StripSettings) -> Edge {
    if placement.top {
        Edge::Top
    } else {
        Edge::Bottom
    }
}

/// Registers the Strip menu's choices. They apply to every strip open.
pub fn install(cx: &mut App) {
    cx.on_action(|_: &about::StripAlongBottom, cx| change(cx, |strip| strip.top = false));
    cx.on_action(|_: &about::StripAlongTop, cx| change(cx, |strip| strip.top = true));
    cx.on_action(|_: &about::StripAlwaysOnTop, cx| {
        change(cx, |strip| strip.always_on_top = !strip.always_on_top)
    });
    cx.on_action(|_: &about::StripNextDisplay, cx| {
        let displays = cx
            .displays()
            .iter()
            .filter_map(|display| display.uuid().ok().map(|uuid| uuid.to_string()))
            .collect::<Vec<_>>();
        let current = placement().display.or_else(|| {
            cx.primary_display()
                .and_then(|display| display.uuid().ok())
                .map(|uuid| uuid.to_string())
        });
        let next = next_display(&displays, current.as_deref());
        change(cx, move |strip| strip.display = next);
    });
}

/// The display after `current` in `displays`, going round; `None` (the
/// main display) when there is no other.
fn next_display(displays: &[String], current: Option<&str>) -> Option<String> {
    if displays.len() < 2 {
        return None;
    }
    let at = current
        .and_then(|current| displays.iter().position(|uuid| uuid == current))
        .unwrap_or(0);
    Some(displays[(at + 1) % displays.len()].clone())
}

fn change(cx: &mut App, choose: impl FnOnce(&mut StripSettings)) {
    let mut chosen = placement();
    choose(&mut chosen);
    let saved = app_settings::application_support_root()
        .and_then(|root| app_settings::save_strip(&root, chosen.clone()));
    if let Err(error) = saved {
        diagnostics::error(format!("could not keep where strips go: {error}"));
    }
    about::set_menus(cx);
    // A window cannot change its kind once open, so each strip is opened
    // again where it now goes.
    let strips = STRIPS.with(|strips| std::mem::take(&mut *strips.borrow_mut()));
    for strip in strips {
        let snapshot = strip.view.read(cx).snapshot().clone();
        let _ = strip
            .window
            .update(cx, |_, window, _| window.remove_window());
        open(strip.document, snapshot, &chosen, cx);
    }
}

/// Forgets strips whose window has closed.
fn prune(cx: &App) {
    let open = cx.windows();
    STRIPS.with(|strips| {
        strips.borrow_mut().retain(|strip| {
            open.iter()
                .any(|window| window.window_id() == strip.window.window_id())
        })
    });
}

/// Shows the World in `document` as a strip, or puts its strip away if it
/// has one.
pub fn toggle(document: &SharedDocument, snapshot: ProjectionSnapshot, cx: &mut App) {
    prune(cx);
    let shown = STRIPS.with(|strips| {
        let mut strips = strips.borrow_mut();
        let at = strips
            .iter()
            .position(|strip| Rc::ptr_eq(&strip.document, document))?;
        Some(strips.remove(at).window)
    });
    match shown {
        Some(window) => {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
        None => open(document.clone(), snapshot, &placement(), cx),
    }
}

/// Keeps any strip of the World in `document` showing it as `projection`
/// does now.
pub fn follow(
    document: &SharedDocument,
    projection: &Entity<world_gpui::ProjectionView>,
    cx: &mut App,
) {
    let views = STRIPS.with(|strips| {
        strips
            .borrow()
            .iter()
            .filter(|strip| Rc::ptr_eq(&strip.document, document))
            .map(|strip| strip.view.clone())
            .collect::<Vec<_>>()
    });
    for view in views {
        if view.read(cx).is_showing(projection.read(cx).snapshot()) {
            continue;
        }
        let snapshot = projection.read(cx).snapshot().clone();
        cx.defer(move |cx| view.update(cx, |view, cx| view.set_snapshot(snapshot, cx)));
    }
}

fn open(
    document: SharedDocument,
    snapshot: ProjectionSnapshot,
    placement: &StripSettings,
    cx: &mut App,
) {
    let display = placement
        .display
        .as_deref()
        .and_then(|chosen| {
            cx.displays()
                .into_iter()
                .find(|display| display.uuid().is_ok_and(|uuid| uuid.to_string() == chosen))
        })
        .or_else(|| cx.primary_display());
    let Some(display) = display else {
        diagnostics::error("could not show a strip: no display");
        return;
    };
    let bounds = strip::band(display.visible_bounds(), edge(placement));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: false,
        show: true,
        // In front of everything, as a panel that does not take the keys
        // from the window being worked in.
        kind: if placement.always_on_top {
            WindowKind::PopUp
        } else {
            WindowKind::Normal
        },
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        display_id: Some(display.id()),
        ..Default::default()
    };
    let for_open = document.clone();
    let title = snapshot.title.clone();
    let opened = cx.open_window(options, move |window, cx| {
        watch_appearance(window);
        window.set_window_title(&title);
        cx.new(|_| {
            StripView::new(snapshot)
                .on_open(move |_, cx| open_world(&for_open, cx))
                .on_close(|window, _| window.remove_window())
        })
    });
    match opened {
        Ok(window) => match window.entity(cx) {
            Ok(view) => STRIPS.with(|strips| {
                strips.borrow_mut().push(Strip {
                    document,
                    window,
                    view,
                })
            }),
            Err(error) => diagnostics::error(format!("could not keep track of a strip: {error}")),
        },
        Err(error) => diagnostics::error(format!("could not show a strip: {error}")),
    }
}

/// Brings the World's own window to the front, opening it again if it was
/// closed.
fn open_world(document: &SharedDocument, cx: &mut App) {
    cx.activate(true);
    let existing = cx.windows().into_iter().find_map(|window| {
        let world = window.downcast::<WorldDocumentView>()?;
        world
            .read(cx)
            .is_ok_and(|view| Rc::ptr_eq(&view.document, document))
            .then_some(world)
    });
    if let Some(world) = existing {
        let _ = world.update(cx, |_, window, _| window.activate_window());
        return;
    }
    let bounds = remembered_window_bounds(RememberedWindow::World, cx)
        .unwrap_or_else(|| Bounds::centered(None, size(px(1100.0), px(900.0)), cx));
    let document = document.clone();
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        move |window, cx| {
            watch_appearance(window);
            cx.new(|cx| WorldDocumentView::with_document(document, cx))
        },
    );
    if let Err(error) = opened {
        diagnostics::error(format!("could not open a World from its strip: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_display_goes_round_and_stays_put_with_one() {
        let two = vec!["a".to_string(), "b".to_string()];
        assert_eq!(next_display(&two, Some("a")).as_deref(), Some("b"));
        assert_eq!(next_display(&two, Some("b")).as_deref(), Some("a"));
        assert_eq!(next_display(&two, None).as_deref(), Some("b"));
        assert_eq!(next_display(&two, Some("gone")).as_deref(), Some("b"));
        assert_eq!(next_display(&["a".to_string()], Some("a")), None);
        assert_eq!(next_display(&[], None), None);
    }
}
