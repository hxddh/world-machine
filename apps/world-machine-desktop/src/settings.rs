//! The Settings window.
//!
//! One place to say how this app's Worlds should speak, separate from the
//! Analyst, because a World's voice is not an Analyst feature — it only lived
//! beside one because that is where the program path already was.
//!
//! The window is deliberately plain and says what is true rather than what is
//! encouraging: which way the voice reaches a model, that a key is the only way
//! a World's contents leave this Mac, and what happens when it is switched off.

use gpui::{
    div, prelude::*, px, size, App, AppContext, Bounds, Context, Entity, IntoElement, Render,
    SharedString, Styled, Window, WindowBounds, WindowOptions,
};
use world_machine_desktop::analyst_settings::{self, VoiceSource};
use world_machine_desktop::key_store;

use crate::diagnostics;
use crate::world_fork::analyst_input::{self, AnalystTextInput};

pub fn open(cx: &mut App) {
    analyst_input::bind_keys(cx);
    let bounds = Bounds::centered(None, size(px(560.0), px(460.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            ..Default::default()
        },
        |_, cx| cx.new(SettingsView::new),
    );
    if let Err(error) = opened {
        diagnostics::error(format!("could not open the Settings window: {error}"));
    }
}

struct SettingsView {
    key_input: Entity<AnalystTextInput>,
    /// Whether a key is stored, read once when the window opens and after every
    /// change. The key itself is never held here.
    key_stored: bool,
    voice_on: bool,
    source: VoiceSource,
    program: Option<String>,
    status: Option<SharedString>,
}

impl SettingsView {
    fn new(cx: &mut Context<Self>) -> Self {
        let key_input = cx.new(|cx| AnalystTextInput::new("Paste an API key…", cx));
        cx.observe(&key_input, |_, _, cx| cx.notify()).detach();
        let mut view = Self {
            key_input,
            key_stored: false,
            voice_on: false,
            source: VoiceSource::Program,
            program: None,
            status: None,
        };
        view.reload();
        view
    }

    /// Read what is actually configured. Anything unreadable reads as nothing
    /// configured, which is the same as never having set it up.
    fn reload(&mut self) {
        self.key_stored = key_store::is_configured();
        let settings = analyst_settings::application_support_root()
            .ok()
            .and_then(|root| analyst_settings::load(&root).ok());
        match settings {
            Some(settings) => {
                self.voice_on = settings.world_voice;
                self.source = settings.world_voice_source.unwrap_or_default();
                self.program = settings.pi_program.map(|path| path.display().to_string());
            }
            None => {
                self.voice_on = false;
                self.source = VoiceSource::Program;
                self.program = None;
            }
        }
    }

    fn apply<F>(&mut self, change: F, cx: &mut Context<Self>)
    where
        F: FnOnce() -> Result<(), String>,
    {
        match change() {
            Ok(()) => self.status = None,
            Err(error) => self.status = Some(error.into()),
        }
        self.reload();
        cx.notify();
    }

    fn set_voice(&mut self, on: bool, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root = analyst_settings::application_support_root()
                    .map_err(|error| error.to_string())?;
                analyst_settings::save_world_voice(&root, on).map_err(|error| error.to_string())
            },
            cx,
        );
    }

    fn set_source(&mut self, source: VoiceSource, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root = analyst_settings::application_support_root()
                    .map_err(|error| error.to_string())?;
                analyst_settings::save_world_voice_source(&root, source)
                    .map_err(|error| error.to_string())
            },
            cx,
        );
    }

    fn save_key(&mut self, cx: &mut Context<Self>) {
        let typed = self.key_input.read(cx).text().to_owned();
        self.apply(move || key_store::save(&typed), cx);
        // Never leave a key sitting in a field once it is stored.
        self.key_input.update(cx, |input, cx| input.clear(cx));
    }

    fn forget_key(&mut self, cx: &mut Context<Self>) {
        self.apply(key_store::clear, cx);
    }

    /// What is true right now, in one sentence.
    fn summary(&self) -> String {
        if !self.voice_on {
            return "Off. Your Worlds read from the copy written into the app, and nothing is sent anywhere.".into();
        }
        match self.source {
            VoiceSource::Program => match self.program.as_deref() {
                Some(program) => format!("On, using {program}."),
                None => "On, but no program is chosen yet, so your Worlds still read from the app's own copy.".into(),
            },
            VoiceSource::Key => {
                if self.key_stored {
                    "On, using your API key.".into()
                } else {
                    "On, but no key is stored yet, so your Worlds still read from the app's own copy.".into()
                }
            }
        }
    }
}

fn button(
    id: &'static str,
    label: impl Into<SharedString>,
    selected: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    let mut control = div()
        .id(id)
        .px_3()
        .p_1()
        .rounded_md()
        .border_1()
        .border_color(crate::theme_rgb(if selected { 0x5e6f91 } else { 0xb8b2a8 }))
        .bg(crate::theme_rgb(if selected { 0xeef2f9 } else { 0xffffff }))
        .text_xs()
        .child(label.into());
    if enabled {
        control = control.cursor_pointer();
    } else {
        control = control.text_color(crate::theme_rgb(0x999990));
    }
    control
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        world_theme::set_dark(matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ));
        window.set_window_title("World Machine Settings");

        let on = self.voice_on;
        let source = self.source;
        let mut switch = div().flex().gap_2().items_center();
        switch = switch.child(
            button("world-voice-off", "Off", !on, true)
                .on_click(cx.listener(|this, _, _, cx| this.set_voice(false, cx))),
        );
        switch = switch.child(
            button("world-voice-on", "On", on, true)
                .on_click(cx.listener(|this, _, _, cx| this.set_voice(true, cx))),
        );

        let mut sources = div().flex().gap_2().items_center();
        sources = sources.child(
            button(
                "world-voice-program",
                "A program on this Mac",
                matches!(source, VoiceSource::Program),
                on,
            )
            .on_click(cx.listener(|this, _, _, cx| this.set_source(VoiceSource::Program, cx))),
        );
        sources = sources.child(
            button(
                "world-voice-key",
                "An API key",
                matches!(source, VoiceSource::Key),
                on,
            )
            .on_click(cx.listener(|this, _, _, cx| this.set_source(VoiceSource::Key, cx))),
        );

        let detail = match source {
            VoiceSource::Program => div().flex().flex_col().gap_1().child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777770))
                    .child(match self.program.as_deref() {
                        Some(program) => format!("Program: {program}"),
                        None => "No program chosen. The Analyst settings are where you point the app at one.".to_string(),
                    }),
            ),
            VoiceSource::Key => {
                let mut key_actions = div().flex().gap_2().items_center();
                key_actions = key_actions.child(
                    button("world-voice-save-key", "Save key", false, on)
                        .on_click(cx.listener(|this, _, _, cx| this.save_key(cx))),
                );
                if self.key_stored {
                    key_actions = key_actions.child(
                        button("world-voice-forget-key", "Forget key", false, true)
                            .on_click(cx.listener(|this, _, _, cx| this.forget_key(cx))),
                    );
                }
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_xs().text_color(crate::theme_rgb(0x777770)).child(
                        if self.key_stored {
                            "A key is stored in your login keychain. Entering another replaces it."
                        } else {
                            "No key stored. It goes into your login keychain, not into any file this app writes."
                        },
                    ))
                    .child(div().w_full().child(self.key_input.clone()))
                    .child(key_actions)
            }
        };

        let mut page = div()
            .size_full()
            .p_5()
            .flex()
            .flex_col()
            .gap_4()
            .bg(crate::theme_rgb(0xfbfbf9))
            .text_color(crate::theme_rgb(0x1f2328))
            .child(div().text_lg().child("World voice"))
            .child(
                div()
                    .text_sm()
                    .text_color(crate::theme_rgb(0x4f5968))
                    .child("When you come back to a World, it can tell you what happened in its own words instead of reading from the copy written into the app. Worlds already open keep the voice they were opened with."),
            )
            .child(div().text_sm().child(self.summary()))
            .child(switch)
            .child(sources)
            .child(detail)
            .child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x777770))
                    .child("An API key is the only setting here that sends anything off this Mac: one request each time you return to a World, carrying what that World has already recorded and nothing else. A program you choose is between you and that program."),
            );
        if let Some(status) = self.status.clone() {
            page = page.child(
                div()
                    .text_xs()
                    .text_color(crate::theme_rgb(0x9a3412))
                    .child(status),
            );
        }
        page
    }
}
