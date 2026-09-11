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

use gpui::{
    canvas, div, fill, hsla, point, prelude::*, px, Bounds, Div, Hsla, Path, Pixels, SharedString,
};
use world_projection::{Fortune, ProjectionSnapshot};

pub(crate) const LINE_HEIGHT: f32 = 96.0;

fn ink() -> Hsla {
    hsla(0.07, 0.14, 0.22, 1.0)
}
fn away() -> Hsla {
    // Strong enough to read as a marked stretch. At 0.16 on a pale ground it
    // was a smudge at the edge of the frame that a person would not notice
    // was there, which is no use for the one thing this band is for.
    hsla(0.11, 0.62, 0.50, 0.30)
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
                move |bounds, _, window, cx| paint(window, bounds, &fortune, since, now, cx),
            )
            .size_full(),
        )
}

#[allow(clippy::too_many_arguments)]
fn paint(
    window: &mut gpui::Window,
    bounds: Bounds<Pixels>,
    fortune: &Fortune,
    since: Option<u64>,
    now: u64,
    cx: &mut gpui::App,
) {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    let pad_x = 16.0;
    let pad_top = 26.0;
    let pad_bottom = 10.0;

    // The reading now is the last point of the line. Without it the silhouette
    // stopped at the final sample and left a bare strip between the end of the
    // shape and the edge of the frame.
    let mut readings = fortune.history.clone();
    if readings.last().is_some_and(|last| last.world_time < now) {
        readings.push(world_projection::FortunePoint {
            world_time: now,
            value: fortune.value,
        });
    }

    let first = readings.first().map(|point| point.world_time).unwrap_or(0);
    let span = (now.saturating_sub(first)).max(1) as f32;
    let peak = readings
        .iter()
        .map(|point| point.value)
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

    if readings.len() >= 2 {
        // Drawn as a filled area rather than a stroke: gpui paths fill, and a
        // silhouette reads as "how much" more directly than a thread does.
        let mut path = Path::new(at(x(readings[0].world_time), y(0)));
        for reading in &readings {
            path.line_to(at(x(reading.world_time), y(reading.value)));
        }
        let last = readings[readings.len() - 1];
        path.line_to(at(x(last.world_time), y(0)));
        path.line_to(at(x(readings[0].world_time), y(0)));
        window.paint_path(path, hsla(0.09, 0.30, 0.38, 0.85));
    }

    // What the number is, and what it is now. A shape with no label is a
    // shape, and the whole point of one figure is that it can be named.
    write(
        window,
        cx,
        bounds,
        fortune.label.clone().into(),
        pad_x,
        7.0,
        0x6b665e,
        11.5,
        gpui::TextAlign::Left,
    );
    write(
        window,
        cx,
        bounds,
        reading(fortune, &readings, since).into(),
        pad_x,
        7.0,
        0x3b3630,
        11.5,
        gpui::TextAlign::Right,
    );

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

/// The reading, and what it averaged over the stretch you were away.
///
/// The average rather than the latest, because the latest is one day and the
/// question a return asks is what the whole absence was like.
fn reading(
    fortune: &Fortune,
    readings: &[world_projection::FortunePoint],
    since: Option<u64>,
) -> String {
    let Some(since) = since else {
        return format!("{} now", fortune.value);
    };
    let away: Vec<i64> = readings
        .iter()
        .filter(|point| point.world_time >= since)
        .map(|point| point.value)
        .collect();
    if away.is_empty() {
        return format!("{} now", fortune.value);
    }
    let mean = away.iter().sum::<i64>() / away.len() as i64;
    format!(
        "{} now · {mean} on average while you were away",
        fortune.value
    )
}

/// One line of words at the top of the band. Right-aligned when it is the
/// reading, left-aligned when it is the label; both are drawn from the same
/// place so they cannot drift apart.
#[allow(clippy::too_many_arguments)]
fn write(
    window: &mut gpui::Window,
    cx: &mut gpui::App,
    bounds: Bounds<Pixels>,
    words: SharedString,
    x: f32,
    y: f32,
    colour: u32,
    size: f32,
    align: gpui::TextAlign,
) {
    let font = window.text_style().font();
    let run = gpui::TextRun {
        len: words.len(),
        font,
        color: crate::theme_rgb(colour).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window
        .text_system()
        .shape_line(words, px(size), &[run], None);
    let width = f32::from(bounds.size.width) - 2.0 * x;

    let _ = line.paint(
        point(bounds.origin.x + px(x), bounds.origin.y + px(y)),
        px(size * 1.3),
        align,
        Some(px(width)),
        window,
        cx,
    );
}
