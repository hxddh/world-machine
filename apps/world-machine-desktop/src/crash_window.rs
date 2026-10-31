//! The crash report, offered on the launch after a crash (1.0 P6).
//!
//! A panic leaves a report file on this computer (`diagnostics.rs`). The
//! next launch, unless the player chose never to be asked, opens this small
//! window once: what happened, that nothing was sent, and what they can do
//! with the report if they want to. "Send Report…" only opens a new e-mail
//! in their own mail program, or, while there is no support address
//! (`shell::SUPPORT_EMAIL`), copies the report and opens a new issue on
//! GitHub for them to paste it into; "Copy Report" puts it on the
//! clipboard; "Show Report File" shows the file. Nothing leaves the
//! computer from here.

use gpui::{
    div, prelude::*, px, size, App, AppContext, Bounds, ClipboardItem, Context, IntoElement,
    Render, Styled, Window, WindowBounds, WindowOptions,
};
use world_gpui::ui;
use world_machine_desktop::app_settings::{self, AppSettings, CrashReports};
use world_machine_desktop::crash_reports::{self, CrashReport, Sending};
use world_machine_desktop::shell::{self, Support};
use world_theme::tokens;

use crate::diagnostics;

/// The reports to offer this launch: none when the player chose never.
pub(crate) fn pending_offer(settings: Option<&AppSettings>) -> Vec<CrashReport> {
    let ask = settings.is_none_or(|settings| settings.crash_reports == CrashReports::Ask);
    match diagnostics::crash_log_dir() {
        Some(dir) if ask => crash_reports::pending(&dir),
        _ => Vec::new(),
    }
}

/// Offers `reports`, once: they are marked offered as the window opens,
/// whatever the player then does.
pub(crate) fn offer(reports: Vec<CrashReport>, cx: &mut App) {
    if reports.is_empty() {
        return;
    }
    if let Some(dir) = diagnostics::crash_log_dir() {
        if let Err(error) = crash_reports::mark_offered(&dir, &reports) {
            diagnostics::error(format!(
                "could not note the crash report as offered: {error}"
            ));
        }
    }
    diagnostics::info(format!("offering {} crash report(s)", reports.len()));
    let bounds = Bounds::centered(None, size(px(520.0), px(360.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |window, cx| {
            crate::watch_appearance(window);
            cx.new(|cx| CrashView {
                reports,
                copied: false,
                copied_for_issue: false,
                never: false,
                focus: cx.focus_handle(),
            })
        },
    );
    if let Err(error) = opened {
        diagnostics::error(format!("could not open the crash report window: {error}"));
    }
}

struct CrashView {
    reports: Vec<CrashReport>,
    copied: bool,
    /// Send Report copied the report for a new GitHub issue.
    copied_for_issue: bool,
    /// The player chose never to be asked again.
    never: bool,
    focus: gpui::FocusHandle,
}

impl CrashView {
    /// The newest report: the one that matters.
    fn newest(&self) -> Option<&CrashReport> {
        self.reports.last()
    }

    fn send(&mut self, cx: &mut Context<Self>) {
        let Some(report) = self.newest() else {
            return;
        };
        match crash_reports::sending(Support::current(), report) {
            Sending::Mail(link) => {
                diagnostics::info("opening a crash report e-mail in the mail program");
                cx.open_url(&link);
            }
            Sending::Issue(text) => {
                diagnostics::info("copying the crash report and opening a new issue");
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                cx.open_url(&diagnostics::issue_url());
                self.copied_for_issue = true;
                cx.notify();
            }
        }
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        if let Some(report) = self.newest() {
            cx.write_to_clipboard(ClipboardItem::new_string(report.text.clone()));
            self.copied = true;
            cx.notify();
        }
    }

    fn reveal(&mut self, cx: &mut Context<Self>) {
        if let Some(report) = self.newest() {
            cx.reveal_path(&report.path);
        }
    }

    fn never_ask(&mut self, cx: &mut Context<Self>) {
        let saved = app_settings::application_support_root()
            .and_then(|root| app_settings::save_crash_reports(&root, CrashReports::Never));
        match saved {
            Ok(()) => self.never = true,
            Err(error) => diagnostics::error(format!("could not keep the choice: {error}")),
        }
        cx.notify();
    }
}

impl Render for CrashView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_rem_size(gpui::px(world_gpui::rem_size()));
        window.set_window_title(&ui::t("Crash Report"));
        if window.focused(cx).is_none() {
            window.focus(&self.focus, cx);
        }
        let file = self
            .newest()
            .map(|report| report.path.display().to_string())
            .unwrap_or_default();
        let copy_label = if self.copied { "Copied" } else { "Copy Report" };
        let mut actions = div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(
                ui::button("crash-send", "Send Report…", ui::ButtonKind::Primary)
                    .on_click(cx.listener(|this, _, _, cx| this.send(cx))),
            )
            .child(
                ui::button("crash-copy", copy_label, ui::ButtonKind::Secondary)
                    .on_click(cx.listener(|this, _, _, cx| this.copy(cx))),
            )
            .child(
                ui::button(
                    "crash-reveal",
                    "Show Report File",
                    ui::ButtonKind::Secondary,
                )
                .on_click(cx.listener(|this, _, _, cx| this.reveal(cx))),
            )
            .child(
                ui::button("crash-not-now", "Not Now", ui::ButtonKind::Secondary)
                    .on_click(cx.listener(|_, _, window, _| window.remove_window())),
            );
        if !self.never {
            actions = actions.child(
                ui::button("crash-never", "Don't Ask Again", ui::ButtonKind::Secondary)
                    .on_click(cx.listener(|this, _, _, cx| this.never_ask(cx))),
            );
        }
        div()
            .id("crash-report")
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
            .child(ui::page_title("World Machine closed unexpectedly"))
            .child(ui::body(
                "A crash report was saved on this computer. Nothing has been sent.",
            ))
            .child(ui::body(Support::current().crash_send_line()))
            .child(ui::caption(file))
            .child(actions)
            .when(self.copied_for_issue, |page| {
                page.child(ui::caption(shell::CRASH_COPIED_FOR_ISSUE))
            })
            .when(self.never, |page| {
                page.child(ui::caption(
                    "You won't be asked again. Reports are still saved; Settings can turn asking back on.",
                ))
            })
    }
}
