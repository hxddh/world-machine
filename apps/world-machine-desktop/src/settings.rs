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
use world_gpui::ui;
use world_machine_desktop::ambience;
use world_machine_desktop::analyst_settings::{self, VoiceSource};
use world_machine_desktop::key_store;
use world_theme::tokens;

use crate::diagnostics;
use crate::world_fork::analyst_input::{self, AnalystTextInput};

pub fn open(cx: &mut App) {
    analyst_input::bind_keys(cx);
    let bounds = Bounds::centered(None, size(px(600.0), px(640.0)), cx);
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
    sound_on: bool,
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
            sound_on: false,
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
                self.sound_on = settings.ambient_sound;
                self.source = settings.world_voice_source.unwrap_or_default();
                self.program = settings.pi_program.map(|path| path.display().to_string());
            }
            None => {
                self.voice_on = false;
                self.sound_on = false;
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

    fn set_sound(&mut self, on: bool, cx: &mut Context<Self>) {
        self.apply(
            move || {
                let root = analyst_settings::application_support_root()
                    .map_err(|error| error.to_string())?;
                analyst_settings::save_ambient_sound(&root, on).map_err(|error| error.to_string())
            },
            cx,
        );
        ambience::set_enabled(self.sound_on);
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

    /// What is true right now, as a few words and how settled it is.
    fn state(&self) -> (&'static str, String) {
        if !self.voice_on {
            return (
                "off",
                "Off. Worlds read from the app's own copy, and nothing leaves this Mac.".into(),
            );
        }
        match self.source {
            VoiceSource::Program => match self.program.as_deref() {
                Some(program) => ("ready", format!("On, through {}.", program_name(program))),
                None => (
                    "incomplete",
                    "On, but no program is chosen, so Worlds still read from the app's copy."
                        .into(),
                ),
            },
            VoiceSource::Key => {
                if self.key_stored {
                    ("ready", "On, through your API key.".into())
                } else {
                    (
                        "incomplete",
                        "On, but no key is stored, so Worlds still read from the app's copy."
                            .into(),
                    )
                }
            }
        }
    }
}

/// The file name of a program, which is what anyone recognises it by.
fn program_name(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
}

/// A switch drawn the way the system draws one: a track and a knob.
fn switch(id: &'static str, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_shrink_0()
        .w(px(38.0))
        .h(px(22.0))
        .p(px(2.0))
        .rounded_full()
        .cursor_pointer()
        .bg(ui::color(if on {
            tokens::ACCENT
        } else {
            tokens::BORDER_STRONG
        }))
        .flex()
        .when(on, |track| track.justify_end())
        .child(
            div()
                .size(px(18.0))
                .rounded_full()
                .bg(ui::color(tokens::SURFACE)),
        )
}

/// One of two ways the voice can reach a model: a tile that says what it is
/// and the one fact that matters about it, selected like a radio button.
fn source_tile(
    id: &'static str,
    glyph: &'static str,
    title: &'static str,
    fact: String,
    selected: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex_1()
        .min_w(px(0.0))
        .p_3()
        .rounded_lg()
        .cursor_pointer()
        .border_1()
        .border_color(ui::color(if selected {
            tokens::ACCENT
        } else {
            tokens::BORDER
        }))
        .bg(ui::color(if selected {
            tokens::ACCENT_SOFT
        } else {
            tokens::SURFACE
        }))
        .hover(|tile| tile.border_color(ui::color(tokens::ACCENT)))
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .size(px(28.0))
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(ui::color(if selected {
                    tokens::ACCENT
                } else {
                    tokens::ROW_HOVER
                }))
                .text_color(ui::color(if selected {
                    tokens::ON_ACCENT
                } else {
                    tokens::TEXT_SECONDARY
                }))
                .text_sm()
                .child(glyph),
        )
        .child(ui::row_title(title))
        .child(ui::caption(fact))
}

/// A grouped block of settings, the way the system groups them.
fn group() -> gpui::Div {
    div()
        .w_full()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(ui::color(tokens::BORDER))
        .bg(ui::color(tokens::SURFACE))
        .flex()
        .flex_col()
        .gap_3()
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

        let voice = group().child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(ui::row_title("Let Worlds speak for themselves"))
                        .child(ui::caption(
                            "Coming back, a World tells you what happened in its own words.",
                        )),
                )
                .child(
                    switch("world-voice-switch", on)
                        .on_click(cx.listener(move |this, _, _, cx| this.set_voice(!on, cx))),
                ),
        );

        let mut page = div()
            .size_full()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .bg(ui::color(tokens::WINDOW))
            .text_color(ui::color(tokens::TEXT))
            .child(ui::page_title("World voice"))
            .child(voice);

        if on {
            let program_fact = match self.program.as_deref() {
                Some(program) => format!("Stays on this Mac · {}", program_name(program)),
                None => "Stays on this Mac · choose one in Analyst settings".into(),
            };
            let key_fact = if self.key_stored {
                "Each return is sent to your provider · key stored".to_string()
            } else {
                "Each return is sent to your provider".to_string()
            };
            let mut sources = group()
                .child(ui::section_label("Where the voice comes from"))
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(
                            source_tile(
                                "world-voice-program",
                                "⌂",
                                "A program on this Mac",
                                program_fact,
                                matches!(source, VoiceSource::Program),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_source(VoiceSource::Program, cx)
                            })),
                        )
                        .child(
                            source_tile(
                                "world-voice-key",
                                "↗",
                                "An API key",
                                key_fact,
                                matches!(source, VoiceSource::Key),
                            )
                            .on_click(
                                cx.listener(|this, _, _, cx| this.set_source(VoiceSource::Key, cx)),
                            ),
                        ),
                );
            if matches!(source, VoiceSource::Key) {
                let mut actions = div().flex().gap_2().items_center().child(
                    ui::button("world-voice-save-key", "Save key", ui::ButtonKind::Primary)
                        .on_click(cx.listener(|this, _, _, cx| this.save_key(cx))),
                );
                if self.key_stored {
                    actions = actions.child(
                        ui::button(
                            "world-voice-forget-key",
                            "Forget key",
                            ui::ButtonKind::Secondary,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.forget_key(cx))),
                    );
                }
                sources = sources
                    .child(div().w_full().child(self.key_input.clone()))
                    .child(actions)
                    .child(ui::caption(
                        "The key goes into your login keychain, never into a file.",
                    ));
            }
            page = page.child(sources);
        }

        let (state, sentence) = self.state();
        let dot = match state {
            "ready" => tokens::SUCCESS,
            "incomplete" => tokens::WARNING,
            _ => tokens::TEXT_TERTIARY,
        };
        page = page.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().size(px(8.0)).rounded_full().bg(ui::color(dot)))
                .child(ui::body(sentence)),
        );
        if let Some(status) = self.status.clone() {
            page = page.child(
                div()
                    .text_sm()
                    .text_color(ui::color(tokens::DANGER))
                    .child(status),
            );
        }
        let sound_on = self.sound_on;
        page.child(ui::caption(
            "Only an API key sends anything off this Mac: what a World has recorded, once per return. Worlds already open keep the voice they opened with.",
        ))
        .child(div().pt_4().child(ui::page_title("Sound")))
        .child(
            group().child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(ui::row_title("Sound"))
                            .child(ui::caption(
                                "The World in front plays its landscape's quiet sound, and a soft tick, bell or chime as cards turn, turns pass and things are built.",
                            )),
                    )
                    .child(
                        switch("ambient-sound-switch", sound_on)
                            .on_click(cx.listener(move |this, _, _, cx| this.set_sound(!sound_on, cx))),
                    ),
            ),
        )
    }
}
