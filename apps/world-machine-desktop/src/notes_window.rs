//! What's New, Known Issues and Acknowledgements: the notes the app
//! carries about itself (`notices.rs`), each in a window of its own, opened
//! from Help, About and the menu bar's Help menu. The notes are in English,
//! as the repository writes them; the windows' own words are translated.

use gpui::{
    div, prelude::*, px, size, App, AppContext, Bounds, Context, IntoElement, Render, Styled,
    Window, WindowBounds, WindowOptions,
};
use world_gpui::ui;
use world_machine_desktop::notices::{self, Block};
use world_theme::tokens;

use crate::diagnostics;

/// Which notes a window shows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Notes {
    WhatsNew,
    KnownIssues,
    Acknowledgements,
}

impl Notes {
    fn title(self) -> &'static str {
        match self {
            Notes::WhatsNew => notices::WHATS_NEW,
            Notes::KnownIssues => notices::KNOWN_ISSUES_TITLE,
            Notes::Acknowledgements => notices::ACKNOWLEDGEMENTS_TITLE,
        }
    }
}

/// Opens a window showing `notes`.
pub(crate) fn open(notes: Notes, cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(620.0), px(620.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |window, cx| {
            crate::watch_appearance(window);
            cx.new(|cx| NotesView {
                notes,
                focus: cx.focus_handle(),
            })
        },
    );
    if let Err(error) = opened {
        diagnostics::error(format!("could not open {}: {error}", notes.title()));
    }
}

struct NotesView {
    notes: Notes,
    /// Holds the keyboard, so Esc closes the window.
    focus: gpui::FocusHandle,
}

/// A note's blocks as elements.
fn note_blocks(blocks: Vec<Block>) -> gpui::Div {
    let mut column = div().flex().flex_col().gap_2();
    for block in blocks {
        column = column.child(match block {
            Block::Heading(text) => div().pt_2().child(ui::heading(text)),
            Block::Paragraph(text) => ui::body(text),
            Block::Bullet(text) => div()
                .flex()
                .gap_2()
                .child(ui::body("•"))
                .child(div().flex_1().min_w(px(0.0)).child(ui::body(text))),
        });
    }
    column
}

/// The third-party notices, by licence: each licence once, with every
/// crate under it.
fn acknowledgements() -> gpui::Div {
    let used = notices::acknowledgements();
    let mut by_licence: Vec<(String, Vec<String>)> = Vec::new();
    for notice in &used {
        let entry = format!("{} {}", notice.name, notice.version);
        match by_licence
            .iter_mut()
            .find(|(licence, _)| *licence == notice.licence)
        {
            Some((_, crates)) => crates.push(entry),
            None => by_licence.push((notice.licence.clone(), vec![entry])),
        }
    }
    by_licence.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    let mut column = div()
        .flex()
        .flex_col()
        .gap_3()
        .child(ui::body(
            "World Machine is built with the work of many others. Each piece of software below is used under the licence named over it.",
        ))
        .child(ui::caption(
            "Every crate in Cargo.lock when the app was built, for every platform.",
        ))
        .child(ui::caption(format!("{} · Cargo.lock", used.len())));
    for line in notices::ASSET_NOTES {
        column = column.child(ui::caption(line));
    }
    for (licence, crates) in by_licence {
        column = column.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(ui::row_title(licence))
                .child(ui::detail(crates.join(", "))),
        );
    }
    column
}

impl Render for NotesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        window.set_window_title(&ui::t(self.notes.title()));
        if window.focused(cx).is_none() {
            window.focus(&self.focus, cx);
        }
        let content = match self.notes {
            Notes::WhatsNew => note_blocks(notices::whats_new()),
            Notes::KnownIssues => div()
                .flex()
                .flex_col()
                .gap_2()
                .child(ui::caption(
                    "Help → Report a Problem… tells us about anything else.",
                ))
                .child(note_blocks(notices::known_issues())),
            Notes::Acknowledgements => acknowledgements(),
        };
        div()
            .id("notes")
            .track_focus(&self.focus)
            .on_key_down(|event: &gpui::KeyDownEvent, window, _| {
                if event.keystroke.key == "escape" {
                    window.remove_window();
                }
            })
            .size_full()
            .overflow_y_scroll()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .child(ui::page_title(self.notes.title()))
            .when(self.notes != Notes::Acknowledgements, |page| {
                page.child(ui::caption("These notes are in English."))
            })
            .child(content)
    }
}
