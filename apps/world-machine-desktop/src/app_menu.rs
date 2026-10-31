//! The in-window app menu, on every platform: Settings, About, Save As,
//! the World codes, Help and Quit, from Home and from a World's window. On
//! a Mac the menu bar has the same items; on Linux and Windows this is the
//! only way to them.
//!
//! The menu is UI state only. Each item does what the menu bar's item
//! does, through the same action, so there is one implementation of each.

#[allow(unused_imports)]
use super::*;
use gpui::{Action, Div, Stateful};
use shell::MenuItem;

/// The menu bar's action for an item.
pub(crate) fn action_for(item: MenuItem) -> Box<dyn Action> {
    match item {
        MenuItem::Settings => Box::new(about::Settings),
        MenuItem::About => Box::new(about::About),
        MenuItem::SaveAs => Box::new(about::SaveWorldAs),
        MenuItem::CopyWorldCode => Box::new(about::CopyWorldCode),
        MenuItem::SaveWorldCode => Box::new(about::SaveWorldCode),
        MenuItem::OpenWorldCode => Box::new(about::OpenWorldCode),
        MenuItem::PasteWorldCode => Box::new(about::PasteWorldCode),
        MenuItem::Help => Box::new(about::ShowHelp),
        MenuItem::Quit => Box::new(about::Quit),
    }
}

/// Does an item that belongs to the whole app (Settings, About, the World
/// codes that open one, Help, Quit) the way the menu bar does: its action,
/// dispatched once the window's turn is over. False for an item the
/// window itself does (Save As, and copying or saving its own code).
pub(crate) fn run_app_item(item: MenuItem, window: &mut Window, cx: &mut App) -> bool {
    match item {
        MenuItem::SaveAs | MenuItem::CopyWorldCode | MenuItem::SaveWorldCode => false,
        _ => {
            window.dispatch_action(action_for(item), cx);
            true
        }
    }
}

/// The button that opens the menu: "≡ Menu", at the window's top right.
pub(crate) fn menu_button(id: &'static str, open: bool) -> Stateful<Div> {
    let label = ui::t("Menu");
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label.clone())
        .aria_expanded(open)
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap_1()
        .px_3()
        .py(px(5.0))
        .rounded_md()
        .text_sm()
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(ui::color(tokens::TEXT))
        .bg(ui::color(tokens::SURFACE))
        .border_1()
        .border_color(ui::color(tokens::BORDER_STRONG))
        .when(open, |button| button.bg(ui::color(tokens::ROW_SELECTED)))
        .hover(|style| style.bg(ui::color(tokens::ROW_HOVER)))
        .cursor_pointer()
        .child("≡")
        .child(label)
}

/// What closes the menu in the window that drew it.
pub(crate) type Close<V> = fn(&mut V, &mut Window, &mut Context<V>);

/// Under the open menu, over everything else in the window: a click
/// anywhere outside the menu closes it, and goes no further (a click on
/// the menu button closes it too, rather than closing and opening it
/// again).
pub(crate) fn backdrop<V: 'static>(close: Close<V>, cx: &mut Context<V>) -> Stateful<Div> {
    div()
        .id("app-menu-backdrop")
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .occlude()
        .on_mouse_down(
            gpui::MouseButton::Left,
            cx.listener(move |this, _, window, cx| {
                close(this, window, cx);
                cx.stop_propagation();
            }),
        )
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |this, _, window, cx| {
                close(this, window, cx);
                cx.stop_propagation();
            }),
        )
}

/// What a click on an item does in the window that drew the menu.
pub(crate) type Pick<V> = Rc<dyn Fn(&mut V, MenuItem, &mut Window, &mut Context<V>)>;

/// The open menu: its items in their groups, the one the keyboard is on
/// (`focused`) ringed, each with its shortcut where it has one. Placed by
/// the window that draws it.
pub(crate) fn menu_list<V: 'static>(
    items: &[MenuItem],
    focused: Option<usize>,
    pick: Pick<V>,
    cx: &mut Context<V>,
) -> Stateful<Div> {
    let mac = cfg!(target_os = "macos");
    let mut list = div()
        .id("app-menu")
        .role(gpui::Role::Menu)
        .aria_label(ui::t("Menu"))
        .w(px(260.0))
        .p_1()
        .flex()
        .flex_col()
        .rounded_lg()
        .border_1()
        .border_color(ui::color(tokens::BORDER))
        .bg(ui::color(tokens::SURFACE))
        .text_color(ui::color(tokens::TEXT))
        .shadow_lg()
        // A click inside the menu stays in it.
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation());
    let mut group = None;
    for (at, item) in items.iter().copied().enumerate() {
        if group != Some(item.group()) {
            if group.is_some() {
                list = list.child(
                    div()
                        .my_1()
                        .h(px(1.0))
                        .w_full()
                        .bg(ui::color(tokens::BORDER)),
                );
            }
            if let Some(heading) = item.group().heading() {
                list = list.child(
                    div()
                        .px_3()
                        .pt_1()
                        .text_xs()
                        .text_color(ui::color(tokens::TEXT_TERTIARY))
                        .child(ui::t(heading)),
                );
            }
            group = Some(item.group());
        }
        let label = ui::t(item.label());
        let is_focused = focused == Some(at);
        let pick = pick.clone();
        list = list.child(
            div()
                .id(item.id())
                .role(gpui::Role::MenuItem)
                .aria_label(label.clone())
                .px_3()
                .py(px(6.0))
                .rounded_md()
                .border_1()
                .border_color(if is_focused {
                    gpui::Hsla::from(ui::color(tokens::ACCENT))
                } else {
                    gpui::transparent_black()
                })
                .when(is_focused, |row| row.bg(ui::color(tokens::ROW_SELECTED)))
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .text_sm()
                .cursor_pointer()
                .hover(|style| style.bg(ui::color(tokens::ROW_HOVER)))
                .child(label)
                .when_some(item.shortcut(mac), |row, keys| {
                    row.child(
                        div()
                            .text_xs()
                            .text_color(ui::color(tokens::TEXT_TERTIARY))
                            .child(keys),
                    )
                })
                .on_click(cx.listener(move |this, _, window, cx| pick(this, item, window, cx))),
        );
    }
    list
}
