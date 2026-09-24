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
