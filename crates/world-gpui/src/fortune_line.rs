//! The shape of what happened, as one line.
//!
//! A return used to be eight cards of equal weight and a list of the most
//! recent events, which for Harbour Town meant five identical rent payments.
//! Eleven weeks of history went past and the moment it turned — the day the
//! school and the bakery both ran out of payroll — was the fourth card of
//! eight, indistinguishable from the rest.
//!
//! A chess engine reports one number and the graph of it across a game shows
//! you where you lost at a glance. This is that, for a World: the figure the
//! Pack reports about itself, drawn across its whole life, with the stretch
//! you were away shaded so a return is a piece of time rather than a list.

use gpui::{canvas, div, fill, hsla, point, prelude::*, px, Bounds, Div, Hsla, Path, Pixels};
use world_projection::{Fortune, ProjectionSnapshot};

pub(crate) const LINE_HEIGHT: f32 = 96.0;

fn ink() -> Hsla {
    hsla(0.07, 0.14, 0.22, 1.0)
}
fn away() -> Hsla {
    hsla(0.11, 0.55, 0.52, 0.16)
}
fn edge() -> Hsla {
    hsla(0.11, 0.20, 0.72, 1.0)
}

/// Whether this World has said enough about itself to be drawn as a shape.
pub(crate) fn has_a_shape(snapshot: &ProjectionSnapshot) -> bool {
    snapshot
        .fortune
        .as_ref()
        .is_some_and(|fortune| fortune.history.len() >= 2)
}

pub(crate) fn line(snapshot: &ProjectionSnapshot) -> Div {
    let Some(fortune) = snapshot.fortune.clone() else {
        return div();
    };
    let since = snapshot
        .briefing
        .as_ref()
        .and_then(|briefing| briefing.since_world_time);
    let now = snapshot.world_time;

    div()
        .w_full()
        .h(px(LINE_HEIGHT))
        .border_b_1()
        .border_color(crate::theme_rgb(0xe2ded6))
        .child(
            canvas(
                move |_, _, _| {},
                move |bounds, _, window, _| paint(window, bounds, &fortune, since, now),
            )
            .size_full(),
        )
}

fn paint(
    window: &mut gpui::Window,
    bounds: Bounds<Pixels>,
    fortune: &Fortune,
    since: Option<u64>,
    now: u64,
) {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let pad_x = 16.0;
    let pad_top = 26.0;
    let pad_bottom = 10.0;

    let first = fortune
        .history
        .first()
        .map(|point| point.world_time)
        .unwrap_or(0);
    let span = (now.saturating_sub(first)).max(1) as f32;
    let peak = fortune
        .history
        .iter()
        .map(|point| point.value)
        .chain(std::iter::once(fortune.value))
        .max()
        .unwrap_or(1)
        .max(1) as f32;

    let x = |world_time: u64| {
        pad_x + (world_time.saturating_sub(first)) as f32 / span * (width - 2.0 * pad_x)
    };
    let y = |value: i64| {
        let top = pad_top;
        let floor = height - pad_bottom;
        floor - (value as f32 / peak) * (floor - top)
    };
    let at = |px_x: f32, px_y: f32| {
        point(
            bounds.origin.x + px(px_x.clamp(0.0, width)),
            bounds.origin.y + px(px_y.clamp(0.0, height)),
        )
    };

    // The stretch you were away, shaded. A return is a piece of time.
    if let Some(since) = since {
        let from = x(since);
        window.paint_quad(fill(
            Bounds::new(
                point(bounds.origin.x + px(from), bounds.origin.y + px(pad_top)),
                gpui::size(
                    px((width - pad_x - from).max(0.0)),
                    px(height - pad_top - pad_bottom),
                ),
            ),
            away(),
        ));
    }

    // The floor, so a line at zero is visibly at zero rather than simply
    // absent — a town with nothing changing hands is the thing most worth
    // being able to see.
    window.paint_quad(fill(
        Bounds::new(
            point(bounds.origin.x + px(pad_x), bounds.origin.y + px(y(0))),
            gpui::size(px(width - 2.0 * pad_x), px(1.0)),
        ),
        edge(),
    ));

    if fortune.history.len() >= 2 {
        // Drawn as a filled area rather than a stroke: gpui paths fill, and a
        // silhouette reads as "how much" more directly than a thread does.
        let mut path = Path::new(at(x(fortune.history[0].world_time), y(0)));
        for reading in &fortune.history {
            path.line_to(at(x(reading.world_time), y(reading.value)));
        }
        let last = fortune.history[fortune.history.len() - 1];
        path.line_to(at(x(last.world_time), y(0)));
        path.line_to(at(x(fortune.history[0].world_time), y(0)));
        window.paint_path(path, hsla(0.09, 0.30, 0.38, 0.85));
    }

    // Where the reader is standing.
    window.paint_quad(fill(
        Bounds::new(
            point(
                bounds.origin.x + px(x(now) - 1.0),
                bounds.origin.y + px(pad_top),
            ),
            gpui::size(px(1.5), px(height - pad_top - pad_bottom)),
        ),
        ink(),
    ));
}
