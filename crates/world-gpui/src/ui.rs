//! The small set of pieces every World surface is built from.
//!
//! A screen assembled from raw `div`s with a colour literal on each one drifts:
//! the same "secondary text" ends up in four greys, every clickable thing is a
//! bordered box that looks like every non-clickable one, and nothing answers
//! the pointer. These helpers fix one type scale, one set of colour roles
//! (`world_theme::tokens`), and one way of looking pressable, so a new panel
//! looks like the rest of the app by default.

use gpui::{
    div, prelude::*, px, relative, rgb, Div, ElementId, FontWeight, Rgba, SharedString, Stateful,
};
use world_theme::tokens::{self, Token};

/// The colour for a token in the current appearance.
pub fn color(token: Token) -> Rgba {
    rgb(token.hex())
}

/// The largest text on a screen: what this page is about.
pub fn page_title(text: impl Into<SharedString>) -> Div {
    div()
        .text_2xl()
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(relative(1.25))
        .text_color(color(tokens::TEXT))
        .child(text.into())
}

/// A heading inside a page: a panel title or a story beat.
pub fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_base()
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(relative(1.35))
        .text_color(color(tokens::TEXT))
        .child(text.into())
}

/// A short heading for a row or card.
pub fn row_title(text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .line_height(relative(1.4))
        .text_color(color(tokens::TEXT))
        .child(text.into())
}

/// Running text under a heading.
pub fn body(text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .line_height(relative(1.55))
        .text_color(color(tokens::TEXT_SECONDARY))
        .child(text.into())
}

/// Small supporting text: a row's second line.
pub fn detail(text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .line_height(relative(1.5))
        .text_color(color(tokens::TEXT_SECONDARY))
        .child(text.into())
}

/// A section label: names a region without competing with its content.
pub fn section_label(text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color(tokens::TEXT_TERTIARY))
        .child(text.into())
}

/// Timestamps, counts, and other text that should recede.
pub fn caption(text: impl Into<SharedString>) -> Div {
    div()
        .text_xs()
        .text_color(color(tokens::TEXT_TERTIARY))
        .child(text.into())
}

/// A raised card on the window.
pub fn card() -> Div {
    div()
        .bg(color(tokens::SURFACE))
        .border_1()
        .border_color(color(tokens::BORDER))
        .rounded_lg()
        .p_4()
}

/// A one-pixel rule between regions.
pub fn divider() -> Div {
    div().w_full().h(px(1.0)).bg(color(tokens::BORDER))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ButtonKind {
    /// The one thing this screen most wants you to do.
    Primary,
    /// Anything else you can do here.
    Secondary,
}

/// A control that looks pressable and answers the pointer.
pub fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
) -> Stateful<Div> {
    let base = div()
        .id(id)
        .flex_shrink_0()
        .px_3()
        .py(px(6.0))
        .rounded_md()
        .text_sm()
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .child(label.into());
    match kind {
        ButtonKind::Primary => base
            .bg(color(tokens::ACCENT))
            .text_color(color(tokens::ON_ACCENT))
            .hover(|style| style.bg(color(tokens::ACCENT_HOVER))),
        ButtonKind::Secondary => base
            .bg(color(tokens::SURFACE))
            .border_1()
            .border_color(color(tokens::BORDER_STRONG))
            .text_color(color(tokens::TEXT))
            .hover(|style| style.bg(color(tokens::ROW_HOVER))),
    }
}

/// A selectable row in a list or sidebar.
pub fn list_row(id: impl Into<ElementId>, selected: bool) -> Stateful<Div> {
    let row = div()
        .id(id)
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .flex()
        .flex_col()
        .gap(px(2.0));
    if selected {
        row.bg(color(tokens::ROW_SELECTED))
    } else {
        row.hover(|style| style.bg(color(tokens::ROW_HOVER)))
    }
}

/// Up to two initials for a name: "Nia Chen" → "NC", "K-88 Radio" → "KR".
pub fn initials(name: &str) -> String {
    let words = name
        .split(|c: char| c.is_whitespace() || c == '-' || c == '·' || c == '↔')
        .filter_map(|word| word.chars().find(|c| c.is_alphanumeric()))
        .collect::<Vec<_>>();
    match words.as_slice() {
        [] => "?".into(),
        [only] => only.to_uppercase().collect(),
        [first, .., last] => first.to_uppercase().chain(last.to_uppercase()).collect(),
    }
}

/// A person's face until there are faces: their initials on a colour that
/// is always theirs.
pub fn avatar(name: &str, size: f32) -> Div {
    let (background, foreground) = tokens::avatar(tokens::seed(name));
    div()
        .flex_shrink_0()
        .size(px(size))
        .rounded_full()
        .bg(color(background))
        .text_color(color(foreground))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px((size * 0.38).round()))
        .font_weight(FontWeight::SEMIBOLD)
        .child(initials(name))
}

/// A small landscape that stands for one World. Its name picks the colours
/// and its identity the shape of the hills and where the sun sits, so a
/// Mars colony and a 1987 town from the same Pack never look alike, and the
/// same World always looks the same.
pub fn cover(name: &str, identity: &str) -> gpui::Canvas<()> {
    let hue = (tokens::seed(name) % 360) as f32 / 360.0;
    let shape = tokens::seed(identity);
    let dark = world_theme::is_dark();
    let (sky_top, sky_bottom, far, near, sun) = if dark {
        (
            gpui::hsla(hue, 0.30, 0.20, 1.0),
            gpui::hsla(hue, 0.28, 0.28, 1.0),
            gpui::hsla((hue + 0.08) % 1.0, 0.25, 0.32, 1.0),
            gpui::hsla((hue + 0.12) % 1.0, 0.30, 0.24, 1.0),
            gpui::hsla((hue + 0.5) % 1.0, 0.55, 0.70, 0.9),
        )
    } else {
        (
            gpui::hsla(hue, 0.45, 0.90, 1.0),
            gpui::hsla(hue, 0.40, 0.82, 1.0),
            gpui::hsla((hue + 0.08) % 1.0, 0.26, 0.74, 1.0),
            gpui::hsla((hue + 0.12) % 1.0, 0.28, 0.60, 1.0),
            gpui::hsla((hue + 0.5) % 1.0, 0.75, 0.80, 0.95),
        )
    };
    landscape([sky_top, sky_bottom, far, near, sun], shape)
}

/// A World's own landscape, in the colours its Pack chose for it: red dust
/// for a Mars colony, a street at night for 1987. Art rather than interface,
/// so it looks the same in either appearance.
pub fn scenery_cover(scenery: &world_projection::Scenery, identity: &str) -> gpui::Canvas<()> {
    let colour = |hex: u32| -> gpui::Hsla { gpui::rgb(hex).into() };
    landscape(
        [
            colour(scenery.sky_top),
            colour(scenery.sky_bottom),
            colour(scenery.far),
            colour(scenery.near),
            colour(scenery.sun),
        ],
        tokens::seed(identity),
    )
}

/// A cover in a World's own colours when its Pack gives them, and in
/// colours picked from its name otherwise.
pub fn cover_for(
    scenery: Option<&world_projection::Scenery>,
    name: &str,
    identity: &str,
) -> gpui::Canvas<()> {
    match scenery {
        Some(scenery) => scenery_cover(scenery, identity),
        None => cover(name, identity),
    }
}

/// Sky, sun and two ridges; `shape` places the sun and shapes the hills.
fn landscape(colours: [gpui::Hsla; 5], shape: u64) -> gpui::Canvas<()> {
    let [sky_top, sky_bottom, far, near, sun] = colours;
    let bit = |shift: u32, range: f32| ((shape >> shift) & 0xff) as f32 / 255.0 * range;
    let sun_x = 0.2 + bit(0, 0.6);
    let far_rise = 0.42 + bit(8, 0.18);
    let far_fall = 0.50 + bit(16, 0.18);
    let near_rise = 0.62 + bit(24, 0.14);
    let near_fall = 0.70 + bit(32, 0.14);
    gpui::canvas(
        |_, _, _| (),
        move |bounds: gpui::Bounds<gpui::Pixels>, _, window, _| {
            use gpui::{point, px, PathBuilder};
            let o = bounds.origin;
            let w = bounds.size.width;
            let h = bounds.size.height;
            let at = |x: f32, y: f32| point(o.x + w * x, o.y + h * y);
            window.paint_quad(gpui::fill(
                bounds,
                gpui::linear_gradient(
                    180.0,
                    gpui::linear_color_stop(sky_top, 0.0),
                    gpui::linear_color_stop(sky_bottom, 1.0),
                ),
            ));
            let radius = f32::from(h) * 0.14;
            let centre = at(sun_x, 0.30);
            window.paint_quad(gpui::quad(
                gpui::Bounds::new(
                    point(centre.x - px(radius), centre.y - px(radius)),
                    gpui::size(px(radius * 2.0), px(radius * 2.0)),
                ),
                px(radius),
                sun,
                px(0.0),
                sun,
                gpui::BorderStyle::default(),
            ));
            for (rise, fall, colour) in [(far_rise, far_fall, far), (near_rise, near_fall, near)] {
                let mut hill = PathBuilder::fill();
                hill.move_to(at(0.0, rise));
                hill.curve_to(at(0.5, (rise + fall) / 2.0), at(0.25, rise - 0.16));
                hill.curve_to(at(1.0, fall), at(0.75, fall + 0.12));
                hill.line_to(at(1.0, 1.0));
                hill.line_to(at(0.0, 1.0));
                hill.close();
                if let Ok(path) = hill.build() {
                    window.paint_path(path, colour);
                }
            }
        },
    )
}

/// How long something new takes to settle into place.
pub const ENTRANCE: std::time::Duration = std::time::Duration::from_millis(620);

/// An easing for the `index`th of several things arriving together: each
/// waits a little longer than the one before it, then settles quickly and
/// softly, so a list reads as arriving in order rather than all at once.
pub fn staggered(index: usize) -> impl Fn(f32) -> f32 {
    let delay = (index as f32 * 0.11).min(0.55);
    move |t| {
        if t <= delay {
            return 0.0;
        }
        let u = ((t - delay) / (1.0 - delay)).clamp(0.0, 1.0);
        1.0 - (1.0 - u).powi(4)
    }
}

/// Something new arriving: it fades in and rises the last few pixels into
/// place. Keyed by `key`, so it plays again only when what it shows changes.
/// Honours the system's reduce-motion setting, as every GPUI animation does.
pub fn arrive<E>(
    element: E,
    key: impl Into<SharedString>,
    index: usize,
) -> gpui::AnimationElement<E>
where
    E: IntoElement + Styled + 'static,
{
    use gpui::{Animation, AnimationExt};
    element.with_animation(
        ElementId::Name(key.into()),
        Animation::new(ENTRANCE).with_easing(staggered(index)),
        |element, t| element.opacity(t).mt(px((1.0 - t) * 10.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::{initials, staggered};

    #[test]
    fn later_arrivals_wait_their_turn_and_everything_settles() {
        let first = staggered(0);
        let fourth = staggered(3);
        assert_eq!(fourth(0.2), 0.0);
        assert!(first(0.2) > 0.0);
        for ease in [first, fourth, staggered(20)] {
            assert!((ease(1.0) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn initials_take_the_first_and_last_word() {
        assert_eq!(initials("Nia Chen"), "NC");
        assert_eq!(initials("Kestrel"), "K");
        assert_eq!(initials("K-88 Radio"), "KR");
        assert_eq!(initials("Nia ↔ Tomas"), "NT");
        assert_eq!(initials(""), "?");
    }
}
